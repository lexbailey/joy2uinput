/*

Hi!

If you're reading this for the first time, welcome!

What does this file do?

    This is the main program file for joy2uinput.
    It reads the config files, then opens a virtual input device via uinput, then
    starts listening to all the joystick input devices and mapping them to the
    virtual input device as per the config.

*/

#![allow(non_upper_case_globals)]
use std::path::PathBuf;
use std::fmt::Debug;
use inotify::{ Inotify, WatchMask, EventMask };
use std::sync::mpsc::Sender;
use std::sync::{Arc,Mutex};
use std::ffi::OsString;
use std::collections::HashMap;
use std::thread::JoinHandle;
use std::path::Path;
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::time::Duration;
mod map_config;
use map_config::{JDEv, JoyInput, Target};
use joydev::GenericEvent;
use std::fs::File;
use std::rc::Rc;
use std::io::BufRead;
use evdev::{InputEvent, EventType};

const conf_dir_env_var: &'static str = "JOY2UINPUT_CONFDIR";

fn get_user_conf_dir() -> Option<PathBuf>{
    if let Some(d) = std::env::var_os(conf_dir_env_var){
        let dir = PathBuf::from(&d);
        if !dir.is_dir(){
            println!("Warning: {} does not point to a directory. No user config will be loaded.", conf_dir_env_var);
            return None;
        }
        return Some(dir);
    }
    if let Some(d) = std::env::var_os("XDG_CONFIG_HOME"){
        if d != ""{
            let mut dir = PathBuf::from(&d);
            dir.push("joy2uinput");
            if dir.is_dir(){
                return Some(dir);
            }
        }
    }
    if let Some(mut home) = dirs::home_dir(){
        home.push(".config/joy2uinput/");
        if home.is_dir(){
            return Some(home);
        }
    }
    let dir = PathBuf::from("/opt/joy2uinput/");
    if dir.is_dir(){
        return Some(dir);
    }
    None
}

enum Ev{
    Joy(u32, joydev::Event),
    Connect(OsString, u32),
    Disconnect(u32),
    Listen(),
    RawEvent(EventType, u16, i32),
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
                        let nl = n.to_string_lossy();
                        if nl.starts_with("js"){
                            let mut path = PathBuf::from("/dev/input");
                            path.push(n);
                            let id: Result<u32,_> = nl[2..].parse();
                            match id{
                                Err(e) => {
                                    eprintln!("Internal error during joypad hotplug handler. This is a bug! {}", e);
                                },
                                Ok(id) => {
                                    let res = match event.mask {
                                        EventMask::CREATE => { evs.send(Ev::Connect(path.into(), id)) },
                                        EventMask::ATTRIB => { evs.send(Ev::Connect(path.into(), id)) },
                                        EventMask::DELETE => { evs.send(Ev::Disconnect(id)) },
                                        _ => unreachable!()
                                    };
                                    if let Err(e) = res {
                                        eprintln!("Internal error during joypad hotplug handler. This is a bug! {}", e);
                                    }
                                }
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

fn pad_thread(evs: Sender<Ev>, s: &Path) -> std::io::Result<(String, std::fs::File, JoinHandle<()>)> {
    let fd = OpenOptions::new().read(true).open(s)?;
    let rfd = fd.as_raw_fd();
    let name = joydev::io_control::get_identifier(rfd).unwrap_or("unknown".to_string());
    let sl = s.to_string_lossy();
    assert!(sl.starts_with("/dev/input/js"));
    let id: Result<u32, _> = sl[13..].parse();
    match id {
        Err(e) => {
            eprintln!("Internal error in joypad handler thread. This is a bug! {}", e);
            Err(std::io::Error::new(std::io::ErrorKind::Other, "Unable to parse ID"))
        },
        Ok(id) => {
            let _ = evs.send(Ev::Println(format!("Device connected: {}", name)));
            Ok((name.clone(), fd, std::thread::spawn(move ||{
                loop{
                    match joydev::io_control::get_event(rfd){
                        Ok(ev) => {
                            if let Err(e) = evs.send(Ev::Joy(id, ev)){
                                eprintln!("Internal error in joypad handler thread. This is a bug! {}", e);
                            }
                        },
                        _ => {break;}
                    }
                }
                let _ = evs.send(Ev::Println(format!("Device disconnected: {}", name)));
            })))
        },
    }
}

fn listen_after(evs: Sender<Ev>, msecs: u64) -> JoinHandle<()> {
    std::thread::spawn(move ||{
        std::thread::sleep(Duration::from_millis(msecs));
        if let Err(_) = evs.send(Ev::Listen()){
            eprintln!("Internal error while waiting to start. This is a bug!");
        }
    })
}

fn read_mappings(path: &PathBuf, mappings: &mut HashMap<OsString, HashMap<JDEv, JoyInput>>) -> bool{
    let mut success = true;
    if let Ok(dir) = std::fs::read_dir(path){
        for f in dir{
            match f{
                Err(_) => {},
                Ok(f) => {
                    if let Ok(ft) = f.file_type(){
                        if ft.is_file(){
                            if f.path().extension() != Some(&std::ffi::OsStr::new("j2umap")){
                                continue;
                            }
                            let mut this_map = HashMap::new();
                            let path = f.path();
                            if !mappings.contains_key(path.file_name().unwrap().into()){ // only if not already loaded this joypad
                                if let Ok(file) = OpenOptions::new().read(true).open(&path) {
                                    let mut line_num = 0;
                                    for line in std::io::BufReader::new(file).lines(){
                                        line_num += 1;
                                        match line {
                                            Ok(line) =>{
                                                let t = line.trim();
                                                if t.len() == 0{ continue; }
                                                if t.starts_with("#"){ continue; }
                                                let m = t.parse::<map_config::Mapping>();
                                                match m{
                                                    Ok(m) => {this_map.insert(m.from, m.to);},
                                                    Err(e) => {
                                                        println!("Error ('{}' line {}): {}", path.display(), line_num, e);
                                                        success = false;
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                println!("Failed to read line from config file: {}", e);
                                                success = false;
                                            },
                                        }
                                    }
                                    mappings.insert(path.file_name().unwrap().into(), this_map);
                                }
                            }
                        }
                    }
                },
            }
        }
    }
    success
}

fn read_config(path: &PathBuf) -> (Option<HashMap<JoyInput, Target>>, bool){
    let mut success = true;
    let mut conf_file = path.clone();
    conf_file.push("joy2uinput.conf");
    if conf_file.is_file(){
        match OpenOptions::new().read(true).open(&conf_file) {
            Err(e) => {
                println!("Error while reading config file {}: {}", &conf_file.display(), e);
                success = false;
            },
            Ok(f) => {
                let mut map = HashMap::new();
                let mut line_num = 0;
                for line in std::io::BufReader::new(f).lines(){
                    line_num += 1;
                    match line {
                        Ok(line) =>{
                            let t = line.trim();
                            if t.len() == 0{ continue; }
                            if t.starts_with("#"){ continue; }
                            let m = t.parse::<map_config::TargetMapping>();
                            match m{
                                Ok(m) => {map.insert(m.from, m.to);},
                                Err(e) => {
                                    println!("Error ('{}' line {}): {}", &conf_file.display(), line_num, e);
                                    success = false;
                                }
                            }
                        },
                        Err(e) => {
                            println!("Failed to read line from config file: {}", e);
                            success = false;
                        },
                    }
                }
                return (Some(map), success);
            }
        }
    }
    (None, success)
}

enum Fatal{ Msg(String) }

impl Debug for Fatal{
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self{
            Fatal::Msg(s) => write!(f, "{}", s),
        }
   } 
}


// joydev control id (the number of a button or axis)
#[derive(Debug,Eq, Hash, PartialEq)]
enum JDCId{
    Button(u8),
    AxisAsButton(u8,i16),
    Axis(u8),
}

#[derive(Debug)]
struct ConnectedPad{
    #[allow(dead_code)] // because we don't want to drop the File
    file: File,
    mapping: Rc<HashMap<JDCId, (JDEv,Target)>>,
    join: JoinHandle<()>,
}

impl From<&JDEv> for JDCId{
    fn from(e: &JDEv) -> Self {
        match e {
            JDEv::Button(n) => JDCId::Button(*n),
            JDEv::AxisAsButton(n,v) => JDCId::AxisAsButton(*n,*v),
            JDEv::Axis(n,_,_) => JDCId::Axis(*n),
        }
    }
}

impl From<std::io::Error> for Fatal {
    fn from(e: std::io::Error) -> Self {
        Fatal::Msg(format!("{}", e))
    }
}


fn launch(args: &Vec<String>){
    let c = &args[0];
    let res = std::process::Command::new(c)
        .stdout(std::process::Stdio::null())
        .stderr(std::process::Stdio::null())
        .args(&args[1..])
        .spawn();
    if let Err(e) = res{
        eprintln!("Failed to launch program: {:?}\n{}", args, e);
    }
    println!("Launched {:?}", args);
}

fn wrapped_main<A>(mut stdout: A, _args: &Vec<String>) -> Result<(),Fatal> where A: std::io::Write  + std::marker::Send + 'static  {

    macro_rules! println {
        () => { write!(stdout, "\n"); };
        ($fstr:literal) => {{ let _res = write!(stdout, concat!($fstr, "\n")); }};
        ($fstr:literal, $($arg:tt)*) => {{ let _res = write!(stdout, concat!($fstr, "\n"), $($arg)*); }};
    }

    let mut pads: HashMap<u32,ConnectedPad> = HashMap::new();
    let mut listening = false;
    let mut _wait_thread = None;
    let mut mappings: HashMap<OsString, HashMap<JDEv, JoyInput>> = HashMap::new();
    let mut expanded_mappings: HashMap<OsString, Rc<HashMap<JDCId, (JDEv, Target)>>> = HashMap::new();

    let mut outmap = None;
    let mut valid = true;
    let mut valid2;

    if let Some(user_conf_dir) = get_user_conf_dir(){
        valid &= read_mappings(&user_conf_dir, &mut mappings);
        (outmap, valid2) = read_config(&user_conf_dir);
        valid &= valid2;
    }

    let default_conf = PathBuf::from("/etc/joy2uinput/");
    if default_conf.is_dir(){
        valid &= read_mappings(&default_conf, &mut mappings);
        if outmap.is_none(){
            (outmap, valid2) = read_config(&default_conf);
            valid &= valid2;
        }
        if outmap.is_none(){
            println!("Error: Unable to find config file joy2uinput.conf in user config dir or default config dir.");
            match get_user_conf_dir(){
                None => {println!("No user config dir searched was found");},
                Some(d) => {println!("User config dir searched was: {}", d.display());},
            }
            println!("Default config dir searched was: /etc/joy2uinput/");
            return Err(Fatal::Msg("No config".to_string()));
        }
    }

    if !valid {
        return Err(Fatal::Msg("Config invalid".to_string()));
    }

    if outmap.is_none(){
        return Err(Fatal::Msg("No output mapping config found. Default config is missing from /etc/joy2uinput/joy2uinput.conf. User config dir also does not contain joy2uinput.conf. See documentation for user config dir search order.".to_string()));
    }

    let outmap = outmap.unwrap();

    for (k,v) in mappings.iter(){
        let mut expmap: HashMap<JDCId, (JDEv, Target)> = HashMap::new();
        for (from, to) in v.iter(){
            if let Some(to) = outmap.get(&to) {
                expmap.insert(from.into(), (from.clone(), to.clone()));
            }
        }
        expanded_mappings.insert(k.clone(), Rc::new(expmap));
    }

    let (send, recv) = std::sync::mpsc::channel::<Ev>();
    let _hp_thread = hotplug_thread(send.clone());
    // enumerate already connected joypads
    match std::fs::read_dir("/dev/input"){
        Err(_) => return Err(Fatal::Msg("Unable to read from /dev/input".to_string())),
        Ok(d) => {
            for f in d{
                if let Ok(j) = f{
                    let n = j.path();
                    let nl = n.to_string_lossy();
                    if nl.starts_with("/dev/input/js"){
                        let id: Result<u32, _> = nl[13..].parse();
                        match id {
                            Err(e) => {
                                eprintln!("Internal error in joypad handler thread. This is a bug! {}", e);
                            },
                            Ok(id) => {
                                if let Err(_) = send.send(Ev::Connect(n.into(), id)) {
                                    eprintln!("Internal error while enumerating joypad devices. This is a bug!");
                                }
                            }
                        }
                    }
                }
            }
        }
    }

    let axis_speeds = Arc::new(Mutex::new(HashMap::<_, i32>::new()));
    let fake_axis_speeds = Arc::new(Mutex::new(HashMap::<_, i32>::new()));

    let mut keys = evdev::AttributeSet::new();
    let mut axes = evdev::AttributeSet::new();
    for (_jp, mapping) in expanded_mappings.iter(){
        for (_from, (_from2, to)) in mapping.iter(){
            match to{
                Target::Key(k) => {
                    keys.insert(k.uinput_key());
                }
                Target::Axis(a) => {
                    let akeys = a.uinput_keys();
                    for key in akeys{
                        keys.insert(key);
                    }
                    let aaxes = a.uinput_axis();
                    if let Some(axis) = aaxes{
                        axes.insert(axis);
                        axis_speeds.lock().unwrap().insert(axis.0, 0);
                    }
                }
                Target::ToggleEnabled() => {}
                Target::Launch(_) => {}
            }
        }
    }

    let uinput_dev: Result<evdev::uinput::VirtualDevice,std::io::Error> = (||->_{
        Ok(evdev::uinput::VirtualDeviceBuilder::new()?.name("joy2udev").with_keys(&keys)?.with_relative_axes(&axes)?.build()?)
    })();
    let mut uinput_dev = match uinput_dev {
        Err(e) => { return Err(Fatal::Msg(format!("Unable to create virtual input device via uinput: {}", e))); },
        Ok(a) => { a },
    };

    let poll_axis = Arc::new(Mutex::new(false));
    let (start_poll, recv_start) = std::sync::mpsc::channel::<()>();
    let t_axis_speeds = axis_speeds.clone();
    let t_fake_axis_speeds = fake_axis_speeds.clone();
    let t_poll_axis = poll_axis.clone();
    let t_poll_send = send.clone();

    // thread that consumes zero energy while no axis is moving
    let _axis_poll_thread = std::thread::spawn(move||{
        for _ev in recv_start{
            loop{
                let speeds = {
                    t_axis_speeds.lock().unwrap().clone()
                };
                let ax_speeds = {
                    t_fake_axis_speeds.lock().unwrap().clone()
                };
                for (axis, speed) in speeds.iter() {
                    if let Err(e) = t_poll_send.send(Ev::RawEvent(EventType::RELATIVE, *axis, *speed)){
                        eprintln!("Error handling axis input. This is a bug! {}", e);
                    }
                }
                for ((neg, pos), speed) in ax_speeds.iter() {
                    if let Err(e) = (||->Result<(),std::sync::mpsc::SendError<_>>{
                        if speed < &0 {
                            t_poll_send.send(Ev::RawEvent(EventType::KEY, *neg, 1))?;
                            t_poll_send.send(Ev::RawEvent(EventType::KEY, *neg, 0))?;
                        }
                        else if speed > &0{
                            t_poll_send.send(Ev::RawEvent(EventType::KEY, *pos, 1))?;
                            t_poll_send.send(Ev::RawEvent(EventType::KEY, *pos, 0))?;
                        }
                        Ok(())
                    })(){
                        eprintln!("Error handling axis input. This is a bug! {}", e);
                    }
                }
                std::thread::sleep(Duration::from_millis(20));
                if !(*t_poll_axis.lock().unwrap()){
                    break;
                }
            }
        }
    });

    macro_rules! set_speed {
        ($code:expr, $delta:expr) => {
            {
                axis_speeds.lock().unwrap().insert($code.0, $delta);
            }
            if !*poll_axis.lock().unwrap() {
                *poll_axis.lock().unwrap() = true;
                if let Err(e) = start_poll.send(()){
                    println!("Internal error: axis event input sender failed. This is a bug! {}", e);
                }
            }
        }
    }

    let mut enabled = true;

    loop{
        match recv.recv(){
            Ok(msg) => match msg {
                Ev::Connect(s, id) => {
                    listening = false;
                    if !pads.contains_key(&id){
                        let t = pad_thread(send.clone(), &Path::new(&s));
                        match t{
                            Ok((name, file, join)) => {
                                let mapping = expanded_mappings.get(&map_config::jpname_to_filename(&name)).cloned();
                                if mapping.is_none(){
                                    println!("Warning: There is no mapping file for the joypad: {}", name);
                                    println!("No inputs will be handled for this joypad.");
                                }
                                else{
                                    let mapping = mapping.unwrap();
                                    pads.insert(id,ConnectedPad{
                                        file,
                                        mapping,
                                        join,
                                    });
                                }
                            }
                            Err(e) => {println!("Error connecting to joypad {}, will retry if device file attributes change...", e);}
                        }
                    }
                    _wait_thread = Some(listen_after(send.clone(), 200));
                },
                Ev::Disconnect(id) => {
                    let pad = pads.remove(&id);
                    if pad.is_none(){
                        continue;
                    }
                    let _ = pad.unwrap().join.join();
                },
                Ev::Joy(dev, ev) => {
                    if listening {
                        let pad = pads.get(&dev);
                        if pad.is_none(){
                            continue;
                        }
                        let pad = pad.unwrap();
                        match ev.type_() {
                            joydev::EventType::Button | joydev::EventType::ButtonSynthetic => {
                                if let Some((_, target)) = pad.mapping.get(&JDCId::Button(ev.number())){
                                    match target {
                                        Target::Key(k) => {
                                            if enabled{
                                                if let Err(e) = uinput_dev.emit(&[InputEvent::new(EventType::KEY, k.uinput_key().code(), ev.value().into())]){
                                                    println!("Error sending event: {}", e);
                                                }
                                            }
                                        },
                                        Target::Axis(a) => {
                                            if enabled{
                                                let val = ev.value();
                                                let speed = val as f32;
                                                let mult = a.multiplier();
                                                let delta = (speed * mult).round() as i32;
                                                if let Some(code) = a.uinput_axis(){
                                                    set_speed!(code, delta);
                                                } 
                                            }
                                        },
                                        Target::ToggleEnabled() => {
                                            if ev.value() != 0{
                                                enabled = !enabled;
                                            }
                                        },
                                        Target::Launch(args) => {
                                            if ev.value() != 0{
                                                launch(&args);
                                            }
                                        },
                                    }
                                }
                            },
                            joydev::EventType::Axis | joydev::EventType::AxisSynthetic => {
                                match pad.mapping.get(&JDCId::Axis(ev.number())){
                                    Some((JDEv::Axis(_n,min,max), target)) => {
                                        match target {
                                            Target::Axis(a) => {
                                                if enabled{
                                                    let val = ev.value();
                                                    let speed = if val < 0 {(val as f32) / (-*min as f32)} else {(val as f32) / (*max as f32)};
                                                    let mult = a.multiplier();
                                                    let delta = (speed * mult).round() as i32;
                                                    if let Some(code) = a.uinput_axis(){
                                                        set_speed!(code, delta);
                                                    }
                                                    else{
                                                        let keys = a.uinput_keys();
                                                        let neg = keys[0].code();
                                                        let pos = keys[1].code();
                                                        {
                                                            fake_axis_speeds.lock().unwrap().insert((neg,pos), delta);
                                                        }
                                                        if !*poll_axis.lock().unwrap() {
                                                            *poll_axis.lock().unwrap() = true;
                                                            if let Err(e) = start_poll.send(()){
                                                                eprintln!("Internal error: axis event input sender failed. This is a bug! {}", e);
                                                            }
                                                        }

                                                    }
                                                }
                                            }
                                            Target::Key(a) => {
                                                println!("Warning: This axis is mapped to a button? Not sure what that means. Target event dropped: {:?}", a);
                                            },
                                            Target::ToggleEnabled() => {
                                                println!("Warning: This axis is mapped to toggle enabled? Not sure what that means.");
                                            },
                                            Target::Launch(_) => {
                                                println!("Warning: This axis is mapped to launch a program? Not sure what that means.");
                                            },
                                        }
                                        
                                    },
                                    _ => {
                                        match pad.mapping.get(&JDCId::AxisAsButton(ev.number(), ev.value())) {
                                            Some((_, target)) => {
                                                match target {
                                                    Target::Key(k) => {
                                                        let code = k.uinput_key().code();
                                                        // Can't group these because that doesn't work when a mouse press and mouse release event are both sent in one group (I don't know why)
                                                        if let Err(e) = uinput_dev.emit(&[ InputEvent::new(EventType::KEY, code, 1), ]){ println!("Error sending event: {}", e); }
                                                        if let Err(e) = uinput_dev.emit(&[ InputEvent::new(EventType::KEY, code, 0), ]){ println!("Error sending event: {}", e); }
                                                    },
                                                    Target::Axis(a) => {
                                                        println!("Warning: Unable to map this button to its axis target because the device models the button as an axis. Target event dropped: {:?}\nFor an explanation of why this happens, see the github issue here: https://github.com/lexbailey/joy2uinput/issues/2", a);
                                                    },
                                                    Target::ToggleEnabled() => {
                                                        enabled = !enabled;
                                                    }
                                                    Target::Launch(args) => {
                                                        launch(&args);
                                                    }
                                                }
                                            },
                                            _ => {},
                                        }
                                    },
                                }
                            },
                        }
                    }
                },
                Ev::RawEvent(evty, code, value) => {
                    if let Err(e) = uinput_dev.emit(&[
                        InputEvent::new(evty, code, value),
                    ]){
                        println!("Error sending event: {}", e);
                    }
                    if axis_speeds.lock().unwrap().values().all(|&a|a==0){
                        *poll_axis.lock().unwrap() = false;
                    }
                }
                Ev::Listen() => {
                    listening = true;
                }
                Ev::Println(s) => {
                    println!("{}", s);
                }
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

mod test_utils;

#[cfg(test)]
mod test{

    use std::io::Write;
    use serial_test::serial;
    use crate::test_utils::{TestEv, new_virtual_joypad, spawn_main};

    macro_rules! next {
        ($step:ident) => {
            println!("Step {} complete", $step);
            $step += 1;
        }
    }

    #[test]
    #[serial]
    fn test_joypad_input() {
        // 1. create a virtual joypad (js0)
        let mut js0 = Some(new_virtual_joypad("testing_joystick0"));

        // 2. spawn a thread to run main()
        let tmp_dir = tempdir::TempDir::new("tmp_joy2uinput_conf").expect("failed to create temp dir");
        let dir_path = tmp_dir.path();
        std::env::set_var("JOY2UINPUT_CONFDIR", dir_path);
        let mut fpath = std::path::PathBuf::from(dir_path); fpath.push("joy2uinput.conf");
        let mut conf_file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(fpath).unwrap();
        conf_file.write_all(b"up = key(up)\n# test config\n\nleftx = axis(mousex,15)").expect("Failed to write temp config for testing");
        let mut mpath = std::path::PathBuf::from(dir_path); mpath.push(crate::map_config::jpname_to_filename("testing_joystick0"));
        let mut map_file = std::fs::OpenOptions::new().write(true).create(true).truncate(true).open(mpath).unwrap();
        map_file.write_all(b"# test mapping\n\nbutton(1) = up\naxis(0,-32767,32767) = leftx").expect("Failed to write temp mapping for testing");
        let args = vec!["joy2uinput".to_string()];
        let (_stdout_read_thread, recv, _timeout_join_handle) = spawn_main(
            move|stdout:std::os::unix::net::UnixStream|{
                crate::wrapped_main(stdout, &args).unwrap();
            }
        );

        let mut step = 0;

        let mut success = false;

        for ev in recv{
            match ev {
                TestEv::Timeout() => {panic!("Timeout");},
                TestEv::Line(s) => {
                    println!("Got line{}", s);
                    match step {
                        0 => {
                            // 3. check that the output contained details of js0
                            if s.contains("Device connected: testing_joystick0") {
                                next!(step);
                                std::thread::sleep(std::time::Duration::from_millis(500)); // first 200ms of events are discarded, so wait a bit
                                // 4. Press a button on the virtual joypad
                                js0.as_mut().unwrap().emit(&[
                                    evdev::InputEvent::new(evdev::EventType::KEY, evdev::Key::BTN_TRIGGER.code(), 1),
                                    evdev::InputEvent::new(evdev::EventType::KEY, evdev::Key::BTN_TRIGGER.code(), 0),
                                    evdev::InputEvent::new(evdev::EventType::KEY, evdev::Key::BTN_DPAD_RIGHT.code(), 1),
                                    evdev::InputEvent::new(evdev::EventType::KEY, evdev::Key::BTN_DPAD_RIGHT.code(), 0),
                                ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(100)); // wait for events to be handled
                                // 5. Wiggle an axis
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, 10), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, 3000), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, 8000), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, 14000), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, 32768), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, 12000), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, 6000), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, -10), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, -1200), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, -16000), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, -29000), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, -32768), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, -40000), ]).expect("Emit failed"); // linux is allowed to report non-clamped values, program must not crash when it does
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                js0.as_mut().unwrap().emit(&[ evdev::InputEvent::new(evdev::EventType::ABSOLUTE, evdev::AbsoluteAxisType::ABS_X.0, 0), ]).expect("Emit failed");
                                std::thread::sleep(std::time::Duration::from_millis(10));
                                // 6. disconnect the virtual joypad
                                std::thread::sleep(std::time::Duration::from_secs(1));
                                js0 = None;
                            }
                        },
                        1 => {
                            // 7. check that the program did not crash, and gracefully handled the disconnect.
                            if s.contains("Device disconnected: testing_joystick0") {
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
