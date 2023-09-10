/*
What does this file do?

    This is the main program file for joy2u-mapgen.
    It listens for joypad input (for mapping, of course) and keyboard input (for the menu).
    It writes config files to the user config directory.
*/
#![allow(non_upper_case_globals)]
use std::path::PathBuf;
use std::fmt::Debug;
use inotify::{ Inotify, WatchMask, EventMask };
use std::sync::mpsc::Sender;
use std::ffi::OsString;
use std::collections::HashMap;
use std::thread::JoinHandle;
use std::path::Path;
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::io::Write;
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};
use std::io::Read;
use std::os::fd::AsFd;
pub mod map_config;
use map_config::JDEv;

const conf_dir_env_var: &'static str = "JOY2UINPUT_CONFDIR";

fn get_user_conf_dir() -> (PathBuf, bool){
    if let Some(d) = std::env::var_os(conf_dir_env_var){
        return (PathBuf::from(&d), true);
    }
    if let Some(d) = std::env::var_os("XDG_CONFIG_HOME"){
        if d != ""{
            let mut buf = PathBuf::from(&d);
            buf.push("joy2uinput");
            return (buf, false);
        }
    }
    if let Some(mut home) = dirs::home_dir(){
        home.push(".config/joy2uinput/");
        return (home, false)
    }
    (PathBuf::from("/opt/joy2uinput/"), false)
}

enum Fatal{
    Msg(String)
}

impl Debug for Fatal{
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self{
            Fatal::Msg(s) => write!(f, "{}", s),
        }
   } 
}

#[derive(Debug,Default,Clone,Copy)]
struct AxisMotion{
    min: i16,
    max: i16,
    n_events: u64,
}

enum Ev{
    Joy(OsString, joydev::Event),
    JoyAxisSettled(),
    Key(u8),
    Connect(OsString),
    Disconnect(OsString),
    Listen(),
    Println(String),
}

fn hotplug_thread(evs: Sender<Ev>) -> Option<std::thread::JoinHandle<()>> {
    let inotify = match (||->std::io::Result<Inotify>{
                let i = Inotify::init()?;
                i.watches().add("/dev/input", WatchMask::CREATE | WatchMask::DELETE | WatchMask::ATTRIB)?;
                Ok(i)
            })() {
        Ok(a) => { Some(a)},
        Err(_e) => { println!("Warning: failed to start inotify, hotplugging is unavailable"); None},
    };
	
    if let Some(mut inotify) = inotify{
        Some(std::thread::spawn(move || {
            let mut buffer = [0; 1024];
            loop{
                if let Ok(events) = inotify.read_events_blocking(&mut buffer){
                    for event in events{
                        let n = event.name.unwrap();
                        if n.to_string_lossy().starts_with("js"){
                            let mut path = PathBuf::from("/dev/input");
                            path.push(n);
                            let res = match event.mask {
                                EventMask::CREATE => { evs.send(Ev::Connect(path.into())) },
                                EventMask::ATTRIB => { evs.send(Ev::Connect(path.into())) },
                                EventMask::DELETE => { evs.send(Ev::Disconnect(path.into())) },
                                _ => unreachable!()
                            };
                            if let Err(_) = res {
                                println!("Internal error during joypad hotplug handler. This is a bug!");
                            }
                        }
                    }
                }
            }
        }))
    }
    else{
        None
    }
}

fn pad_thread(evs: Sender<Ev>, s: &Path) -> joydev::Result<(String, std::fs::File, JoinHandle<()>)> {
    let fd = OpenOptions::new().read(true).open(s)?;
    let rfd = fd.as_raw_fd();
    let name = joydev::io_control::get_identifier(rfd).unwrap_or("unknown".to_string());
    let p: OsString = PathBuf::from(s).as_os_str().into();
    let _ = evs.send(Ev::Println(format!("Device connected: {}", name)));
    Ok((name.clone(), fd, std::thread::spawn(move ||{
        loop{
            match joydev::io_control::get_event(rfd){
                Ok(ev) => {
                    if let Err(_) = evs.send(Ev::Joy(p.clone(), ev)){
                        println!("Internal error in joypad handler thread. This is a bug!");
                    }
                },
                _ => {break;}
            }
        }
        let _ = evs.send(Ev::Println(format!("Device disconnected: {}", name)));
    })))
}

fn listen_after(evs: Sender<Ev>, msecs: u64) -> JoinHandle<()> {
    std::thread::spawn(move ||{
        std::thread::sleep(Duration::from_millis(msecs));
        if let Err(_) = evs.send(Ev::Listen()){
            println!("Internal error while waiting to start. This is a bug!");
        }
    })
}

fn wrapped_main<A>(mut stdout: A, args: &Vec<String>) -> Result<(),Fatal> where A: std::io::Write {

    macro_rules! println {
        () => { write!(stdout, "\n"); };
        ($fstr:literal) => {{ let _res = write!(stdout, concat!($fstr, "\n")); }};
        ($fstr:literal, $($arg:tt)*) => {{ let _res = write!(stdout, concat!($fstr, "\n"), $($arg)*); }};
    }

    let mut debug_mode = false;
    for arg in &args[1..]{
        if arg == "--debug" {
            debug_mode = true;
        }
        else{
            println!("Warning: ignored arugment: {}", arg);
        }
    }

    if debug_mode{
        println!("joy2u-mapgen (debug mode)");
        println!("dumping all events for connected joypads...");
        println!("");
    }
    else{
        println!("joy2u-mapgen - generate a mapping file for joy2uinput");
        println!("");
    }

    let (user_conf_dir, is_from_env) = get_user_conf_dir();

    if !debug_mode{
        if !user_conf_dir.is_dir() {
            let r = std::fs::create_dir_all(&user_conf_dir);
            match r{
                Ok(_) => {},
                Err(e) => {
                    let mut err = format!("config path is not a directory and could not be made into a directory: {} failed to create directory: {}", user_conf_dir.display(), e);
                    if is_from_env{
                        err += &(format!("\nNote: config path was set by environment variable: {}", conf_dir_env_var));
                    }
                    return Err(Fatal::Msg(err));
                }
            }
        }

        println!("Generated configuration will be output to `{}`", user_conf_dir.display());
        if is_from_env{
            println!("Note: config path was set by environment variable: {}", conf_dir_env_var);
        }
    }

    let (send, recv) = std::sync::mpsc::channel::<Ev>();
    let _hp_thread = hotplug_thread(send.clone());
    // enumerate already connected joypads
    let mut n_pads: usize = 0;
    match std::fs::read_dir("/dev/input"){
        Err(_) => return Err(Fatal::Msg("Unable to read from /dev/input".to_string())),
        Ok(d) => {
            for f in d{
                if let Ok(j) = f{
                    let n = j.path();
                    if n.to_string_lossy().starts_with("/dev/input/js"){
                        if let Err(_) = send.send(Ev::Connect(n.into())) {
                            println!("Internal error while enumerating joypad devices. This is a bug!");
                        }
                        n_pads += 1;
                    }
                }
            }
        }
    }

    let key_sender = send.clone();
    let _keyboard_thread = std::thread::spawn(move||{
        let stdin = std::io::stdin();
        use nix::sys::termios::{self, LocalFlags};
        let attrs = termios::tcgetattr(stdin.as_fd());
        // Make best effort to get terminal into the right mode
        match attrs{
            Err(_) => {return;}
            Ok(mut attrs) => {
                attrs.local_flags.remove(LocalFlags::ICANON | LocalFlags::ECHO);
                let _ = termios::tcsetattr(stdin.as_fd(), nix::sys::termios::SetArg::TCSANOW, &attrs);
            }
        }
        for b in stdin.bytes(){
            match b {
                Err(_) => break,
                Ok(b) => {
                    if let Err(_a) = key_sender.send(Ev::Key(b)){
                        break;
                    }
                }
            }
        }
    });

    let mut pads = HashMap::new();
    
    if !debug_mode{
        println!("");
        println!("{} devices currently connected", n_pads);
        println!("");
        println!("To start generating a config, press any button on the joypad to configure.");
        println!("");
    }

    let mut listening = false;
    let mut _wait_thread = None;

    let mut cur_dev = None;
    let mut mapping_path: Option<PathBuf> = None;

    use map_config::{Button, Axis, JoyInput};

    let mut next_map = 0;
    const to_map: [JoyInput; 18] = [
        JoyInput::Button(Button::Up()),
        JoyInput::Button(Button::Down()),
        JoyInput::Button(Button::Left()),
        JoyInput::Button(Button::Right()),
        
        JoyInput::Button(Button::A()),
        JoyInput::Button(Button::B()),
        JoyInput::Button(Button::X()),
        JoyInput::Button(Button::Y()),

        JoyInput::Button(Button::Start()),
        JoyInput::Button(Button::Select()),
    
        JoyInput::Button(Button::RShoulder()),
        JoyInput::Button(Button::LShoulder()),
        JoyInput::Button(Button::RTrigger()),
        JoyInput::Button(Button::LTrigger()),

        JoyInput::Axis(Axis::LeftX()),
        JoyInput::Axis(Axis::LeftY()),
        JoyInput::Axis(Axis::RightX()),
        JoyInput::Axis(Axis::RightY()),
    ];

    let mut config: HashMap<JDEv,&JoyInput> = HashMap::new();

    let mut recent_axes = HashMap::new();

    let next_motion_check: Arc<Mutex<Instant>> = Arc::new(Mutex::new(Instant::now()));


    let mut motion_check_thread = None;

    macro_rules! wait_to_settle {
        ($timeout:expr) => {
            {
                *next_motion_check.lock().unwrap() = Instant::now()+Duration::from_millis($timeout);
            }
            if motion_check_thread.is_none(){
                let t_send = send.clone();
                let t_next_check = next_motion_check.clone();
                motion_check_thread = Some(std::thread::spawn(move || {
                    loop {
                        let t = { t_next_check.lock().unwrap().clone() };
                        let now = Instant::now();
                        if now > t {
                            let _ = t_send.send(Ev::JoyAxisSettled());
                            break;
                        }
                        else{
                            std::thread::sleep((t-now) + Duration::from_millis(1));
                        }
                    }
                }));
            }
        }
    }

    macro_rules! reset_axes {
        () => { recent_axes.clear(); }
    }

    macro_rules! next {
        ()=>{
            next_map += 1;
            reset_axes!();
            if next_map >= to_map.len(){
                println!("Complete!");
                let filename = mapping_path.as_ref().unwrap();
                let outfile = OpenOptions::new().write(true).create(true).truncate(true).open(filename);
                match outfile {
                    Ok(mut f) => {
                        let mut success = true;
                        if let Err(_) = writeln!(f, "# this joy2uinput mapping file was auto generated by joy2u-mapgen",){
                            success = false;
                        }
                        else{
                            let mut vconf: Vec<_> = config.iter().map(|(f,t)|(t,f)).collect();
                            vconf.sort();
                            for (&to, from) in vconf{
                                if let Err(e) = writeln!(f, "{}", map_config::Mapping{from:from.clone(),to:to.clone()}){
                                    println!("Failed to write to file: {}", e);
                                    success = false;
                                    break;
                                }
                            }
                        }
                        if success {
                            println!("Config file written!");
                            return Ok(())
                        }
                        return Err(Fatal::Msg("Failed".to_string()));
                    },
                    Err(e) => { println!("Failed to write to config file: {}, {}", filename.display(), e); },
                }
            }
            else {
                let n = &to_map[next_map];
                match n {
                    JoyInput::Button(_) => {println!("\nPress {}", n);},
                    JoyInput::Axis(_) => {println!("\nMove axis {} quickly to both extremes, then wait", n);},
                }
            }
        }
    }

    macro_rules! record_axis_event {
        ($ev:ident, $timeout:expr) => {
            let num = $ev.number();
            let val = $ev.value();
            let mut axis = recent_axes.get(&num).map(|a|*a).or(Some(AxisMotion::default())).unwrap();
            axis.min = axis.min.min(val);
            axis.max = axis.max.max(val);
            axis.n_events += 1;
            recent_axes.insert(num, axis);
            wait_to_settle!($timeout);
        }
    }

    macro_rules! get_settled_event {
        () => {
            {
                let mut highest_freq = 0;
                let mut axis = None;
                for m in recent_axes.values(){
                    highest_freq = highest_freq.max(m.n_events);
                }
                for (n,m) in recent_axes.iter(){
                    if m.n_events == highest_freq {
                        axis = Some(n);
                        break;
                    }
                }
                axis.map(|a| (*a, *recent_axes.get(a).unwrap()))
            }
        }
    }

    loop{
        match recv.recv(){
            Ok(msg) => match msg {
                Ev::Connect(s) => {
                    listening = false;
                    if !pads.contains_key(&s){
                        let t = pad_thread(send.clone(), &Path::new(&s));
                        match t{
                            Ok(t) => {pads.insert(s,t);}
                            Err(e) => {println!("Error connecting to joypad {:?}", e);}
                        }
                    }
                    _wait_thread = Some(listen_after(send.clone(), 200));
                },
                Ev::Disconnect(s) => {
                    if let Some((_n, _fd, join)) = pads.remove(&s){
                        let _ = join.join();
                    }
                },
                Ev::Key(b) => {
                    if debug_mode {continue;}
                    match b {
                        b' ' => {
                            // Skip this button
                            next!();
                        },
                        b'q' | b'\x1b' => {
                            // Quit
                            break;
                        },
                        _ => {},
                    }
                },
                Ev::Joy(dev, ev) => {
                    use joydev::GenericEvent;
                    if listening {
                        let pad = pads.get(&dev);
                        if pad.is_none(){
                            continue;
                        }
                        if debug_mode {
                            println!("{}: {}", pad.unwrap().0,
                                match ev.type_(){
                                    joydev::EventType::Button | joydev::EventType::ButtonSynthetic => {
                                        format!("button({}) value {}", ev.number(), ev.value())
                                    },
                                    joydev::EventType::Axis | joydev::EventType::AxisSynthetic => {
                                        format!("axis({}, _, _) value {}", ev.number(), ev.value())
                                    },
                                }
                            );
                            continue;
                        }
                        let (name, _file, _joinhandle) = pad.unwrap();

                        if let Some(cdev) = cur_dev.as_ref(){
                            if &dev == cdev{
                                // Do a mapping thing (maybe)
                                let n = &to_map[next_map];
                                match n{
                                    JoyInput::Button(_) => {
                                        match ev.type_() {
                                            joydev::EventType::Button | joydev::EventType::ButtonSynthetic => {
                                                if ev.value() == 1{
                                                    println!("Button number {} is '{}'", ev.number(), n);
                                                    config.insert(JDEv::Button(ev.number()), n);
                                                    next!();
                                                }
                                            },
                                            joydev::EventType::Axis | joydev::EventType::AxisSynthetic => {
                                                if ev.value() != 0{
                                                    record_axis_event!(ev, 300);
                                                }
                                            }
                                        }
                                    },
                                    JoyInput::Axis(_) => {
                                        match ev.type_() {
                                            joydev::EventType::Button | joydev::EventType::ButtonSynthetic => {}, //ignore buttons if mapping an axis
                                            joydev::EventType::Axis | joydev::EventType::AxisSynthetic => {
                                                record_axis_event!(ev, 600);
                                            }
                                        }
                                    },
                                }
                                //println!("Input from joypad {:#?}: {:?}", dev, ev);
                            }
                        }

                        if cur_dev.is_none(){
                            cur_dev = Some(dev.clone());
                            let mut path = user_conf_dir.clone();
                            path.push(map_config::jpname_to_filename(name));
                            println!("\nStarted mapping joypad: {}", name);
                            if path.is_file() {
                                println!("WARNING: Mapping this joypad will overwrite the existing mapping configuration in '{}'.", path.display());
                            }
                            mapping_path = Some(path);
                            println!("To skip mapping a button, press the spacebar");
                            if next_map < to_map.len(){
                                let n = &to_map[next_map];
                                println!("\nPress {}", n);
                            }
                        }
                    }
                },
                Ev::JoyAxisSettled() => {
                    if debug_mode {continue;}
                    let _ = motion_check_thread.take().unwrap().join();
                    let event = get_settled_event!();
                    if event.is_none(){
                        // spurious event
                        println!("TODO: figure out why this happens and if it's a problem. (If you're seeing this message then ... first of all, hi :D, secondly, I still haven't fixed what might be a race condition. If the program misbehaves after this point, please send a bug report on github. Thanks.)");
                    }
                    else{
                        let (number, motion) = get_settled_event!().unwrap();
                        reset_axes!();
                        let n = &to_map[next_map];
                        match n{
                            JoyInput::Button(_) => {
                                if motion.n_events > 1 {
                                    println!("Detected axis event sequence. Are you sure you pressed a button? Try again.")
                                }
                                else{
                                    let val = if motion.min != 0 {motion.min} else {motion.max};
                                    println!("Axis {} at value of {} is button '{}'", number, val, n);
                                    config.insert(JDEv::AxisAsButton(number, val), n);
                                    next!();
                                }
                            },
                            JoyInput::Axis(_) => {
                                println!("Axis {} is '{}' with range {}..{}", number, n, motion.min, motion.max);
                                config.insert(JDEv::Axis(number, motion.min, motion.max), n);
                                next!();
                            },
                        }
                    }
                },
                Ev::Println(s) => {
                    println!("{}", s);
                }
                Ev::Listen() => {
                    listening = true;
                },
            }
            _ => {break;}
        }
    }

    Ok(())
}

fn main() -> Result<(), Fatal>{
    let args: Vec<String> = std::env::args().collect();
    wrapped_main(std::io::stdout(), &args)
}

#[cfg(test)]
mod test{
    use std::io::{Read,Write,Error,ErrorKind};

    fn new_virtual_joypad(name: &str) -> evdev::uinput::VirtualDevice {
        let mut keys = evdev::AttributeSet::new();
        keys.insert(evdev::Key::BTN_0);
        keys.insert(evdev::Key::BTN_1);
        keys.insert(evdev::Key::BTN_2);
        keys.insert(evdev::Key::BTN_3);
        keys.insert(evdev::Key::BTN_4);
        keys.insert(evdev::Key::BTN_5);
        keys.insert(evdev::Key::BTN_6);
        keys.insert(evdev::Key::BTN_7);
        keys.insert(evdev::Key::BTN_8);
        keys.insert(evdev::Key::BTN_9);
        keys.insert(evdev::Key::BTN_TRIGGER);
        keys.insert(evdev::Key::BTN_DPAD_UP);
        keys.insert(evdev::Key::BTN_DPAD_DOWN);
        keys.insert(evdev::Key::BTN_DPAD_LEFT);
        keys.insert(evdev::Key::BTN_DPAD_RIGHT);

        let ax_LX = evdev::UinputAbsSetup::new(evdev::AbsoluteAxisType::ABS_X, evdev::AbsInfo::new(0, -100, 100, 0, 0, 1));
        let ax_LY = evdev::UinputAbsSetup::new(evdev::AbsoluteAxisType::ABS_Y, evdev::AbsInfo::new(0, -100, 100, 0, 0, 1));
        let ax_RX = evdev::UinputAbsSetup::new(evdev::AbsoluteAxisType::ABS_RX, evdev::AbsInfo::new(0, -100, 100, 0, 0, 1));
        let ax_RY = evdev::UinputAbsSetup::new(evdev::AbsoluteAxisType::ABS_RY, evdev::AbsInfo::new(0, -100, 100, 0, 0, 1));
    
        evdev::uinput::VirtualDeviceBuilder::new()
            .expect("Failed to make uinput device builder")
            .name(name)
            .with_keys(&keys).expect("Failed to add buttons to virtual joystick device")
            .with_absolute_axis(&ax_LX).expect("Failed to set up absolute axes on virtual joystick device")
            .with_absolute_axis(&ax_LY).expect("Failed to set up absolute axes on virtual joystick device")
            .with_absolute_axis(&ax_RX).expect("Failed to set up absolute axes on virtual joystick device")
            .with_absolute_axis(&ax_RY).expect("Failed to set up absolute axes on virtual joystick device")
            .build().expect("Failed to build virtual joystick device")
    }

    #[derive(Debug)]
    enum TestEv{
        Timeout(),
        Line(String),
    }

    fn spawn_main(args: Vec<String>) -> (std::thread::JoinHandle<()>, std::sync::mpsc::Receiver<TestEv>){
        let (send, recv) = std::sync::mpsc::channel();
        let send1 = send.clone();

        let timeout_join = std::thread::spawn(move||{
            std::thread::sleep(std::time::Duration::from_secs(3));
            send1.send(TestEv::Timeout());
        });

        let send2 = send.clone();
        let jh = std::thread::spawn(move||{
            let (mut stdout, mut t_stdout) = std::os::unix::net::UnixStream::pair().unwrap();
            let main_join = std::thread::spawn(move||{
                crate::wrapped_main(t_stdout, &args);
            });

            let mut line = String::new();
            stdout.set_read_timeout(Some(std::time::Duration::from_millis(100)));
            loop {
                let mut b: [u8;100] = [0;100];
                let r = stdout.read(&mut b);
                match r{
                    Err(e) => {
                        let k = e.kind();
                        if k != ErrorKind::Interrupted && k != ErrorKind::WouldBlock && k != ErrorKind::TimedOut{
                            panic!("Failed to read from program stdout: {}", e);
                        }
                    },
                    Ok(len) => {
                        let s = String::from_utf8_lossy(&b[0..len]);
                        for c in s.chars(){
                            if c == '\n'{
                                send2.send(TestEv::Line(line.clone()));
                                line = String::new();
                            }
                            else{
                                line.push(c);
                            }
                        }
                    },
                }
            }
        });
        (jh,recv)
    }

    macro_rules! next {
        ($step:ident) => {
            println!("Step {} complete", $step);
            $step += 1;
        }
    }

    #[test]
    fn test_joypad_debug_mode() {
        // 1. create a virtual joypad (js0)
        let mut js0 = new_virtual_joypad("testing_joystick0");
        let mut js1 = None;

        // 2. spawn a thread to run main()
        let args = vec!["joy2u-mapgen".to_string(), "--debug".to_string()];
        let (stdout_read_thread, mut recv) = spawn_main(args);

        let mut step = 0;

        let mut success = false;

        for ev in recv{
            match ev {
                TestEv::Timeout() => {panic!("Timeout");},
                TestEv::Line(s) => {
                    println!("{}", s);
                    match step {
                        0 => {
                            // 3. wait for the normal output
                            if s.contains("joy2u-mapgen") && s.contains("debug mode") {
                                next!(step);
                            }
                        },
                        1 => {
                            // 4. check that the output contained details of js0
                            if s.contains("Device connected: testing_joystick0") {
                                next!(step);
                                std::thread::sleep(std::time::Duration::from_millis(500)); // first 200ms of events are discarded, so wait a bit
                                // 5. Press a button on the virtual joypad
                                js0.emit(&[
                                    evdev::InputEvent::new(evdev::EventType::KEY, evdev::Key::BTN_TRIGGER.code(), 1),
                                ]).expect("Emit failed");
                            }
                        },
                        2 => {
                            // 6. Check button is reported
                            if s.contains("testing_joystick0: button(0) value 1") {
                                next!(step);
                                // 7. release the button
                                js0.emit(&[
                                    evdev::InputEvent::new(evdev::EventType::KEY, evdev::Key::BTN_TRIGGER.code(), 0),
                                ]).expect("Emit failed");
                            }
                        },
                        3 => {
                            // 8. check button release is reported
                            if s.contains("testing_joystick0: button(0) value 0") {
                                next!(step);
                                // 9. simulate axis input
                                js0.emit(&[
                                    evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, 50),
                                ]).expect("Emit failed");
                            }
                        },
                        4 => {
                            // 10. check axis is reported
                            if s.contains("testing_joystick0: axis(0, _, _) value ") { // not sure what the value will actually be, because apparently some rescaling happens???? can't find uinput docs aobut this, can't be bothered to read kernel source right now
                                next!(step);
                                // 11. axis to rest position
                                js0.emit(&[
                                    evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, 0),
                                ]).expect("Emit failed");
                            }
                        },
                        5 => {
                            // 12. check rest position reported
                            if s.contains("testing_joystick0: axis(0, _, _) value 0") {
                                next!(step);
                                // 13. Connect another joystick
                                js1 = Some(new_virtual_joypad("testing_joystick1"));
                            }
                        },
                        6 => {
                            // 14. check the new device is reported as connected
                            if s.contains("Device connected: testing_joystick1") {
                                next!(step);
                                std::thread::sleep(std::time::Duration::from_millis(500)); // first 200ms of events are discarded, so wait a bit
                                // 15 press a button.
                                js1.as_mut().unwrap().emit(&[
                                    evdev::InputEvent::new(evdev::EventType::KEY, evdev::Key::BTN_TRIGGER.code(), 1),
                                ]).expect("Emit failed");
                            }
                        },
                        7 => {
                            // 16. check button is reported
                            if s.contains("testing_joystick1: button(0) value 1") {
                                next!(step);
                                // 17. disconnect joystick 1
                                js1 = None;
                            }
                        },
                        8 => {
                            // 18. check disconnect is reported
                            if s.contains("Device disconnected: testing_joystick1") {
                                // Test complete
                                success = true;
                                break;
                            }
                        },
                        _ => {panic!("Unexpected step");},
                    }
                },
            }
        }
        assert!(success, "Something didn't happen in the right sequence");
    }
}
