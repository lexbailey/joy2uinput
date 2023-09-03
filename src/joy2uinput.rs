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
    if let Ok(d) = std::env::var(conf_dir_env_var){
        let dir = PathBuf::from(&d);
        if !dir.is_dir(){
            eprintln!("Warning: {} does not point to a directory. No user config will be loaded.", conf_dir_env_var);
            return None;
        }
        return Some(dir);
    }
    if let Ok(d) = std::env::var("XDG_CONFIG_HOME"){
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
}


fn hotplug_thread(evs: Sender<Ev>) -> Option<std::thread::JoinHandle<()>> {
    let inotify = match (||->std::io::Result<Inotify>{
                let i = Inotify::init()?;
                i.watches().add("/dev/input", WatchMask::CREATE | WatchMask::DELETE | WatchMask::ATTRIB)?;
                Ok(i)
            })() {
        Ok(a) => { Some(a)},
        Err(_e) => { eprintln!("Warning: failed to start inotify, hotplugging is unavailable"); None},
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
            eprintln!("Device connected: {}", name);
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
                eprintln!("Device diconnected: {}", name);
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
                                                        eprintln!("Error ('{}' line {}): {}", path.display(), line_num, e);
                                                        success = false;
                                                    }
                                                }
                                            },
                                            Err(e) => {
                                                eprintln!("Failed to read line from config file: {}", e);
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
                eprintln!("Error while reading config file {}: {}", &conf_file.display(), e);
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
                                    eprintln!("Error ('{}' line {}): {}", &conf_file.display(), line_num, e);
                                    success = false;
                                }
                            }
                        },
                        Err(e) => {
                            eprintln!("Failed to read line from config file: {}", e);
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

fn main() -> Result<(),Fatal> {

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
            eprintln!("Error: Unable to find config file joy2uinput.conf in user config dir or default config dir.");
            match get_user_conf_dir(){
                None => {eprintln!("No user config dir searched was found");},
                Some(d) => {eprintln!("User config dir searched was: {}", d.display());},
            }
            eprintln!("Default config dir searched was: /etc/joy2uinput/");
            return Err(Fatal::Msg("No config".to_string()));
        }
    }

    if !valid {
        return Err(Fatal::Msg("Config invalid".to_string()));
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
            }
        }
    }

    let mut uinput_dev = evdev::uinput::VirtualDeviceBuilder::new()?.name("joy2udev").with_keys(&keys)?.with_relative_axes(&axes)?.build()?;

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
                    eprintln!("Internal error: axis event input sender failed. This is a bug! {}", e);
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
                                    eprintln!("Warning: There is no mapping file for the joypad: {}", name);
                                    eprintln!("No inputs will be handled for this joypad.");
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
                            Err(e) => {eprintln!("Error connecting to joypad {}, will retry if device file attributes change...", e);}
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
                                                    eprintln!("Error sending event: {}", e);
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
                                                eprintln!("Warning: This axis is mapped to a button? Not sure what that means. Target event dropped: {:?}", a);
                                            },
                                            Target::ToggleEnabled() => {
                                                eprintln!("Warning: This axis is mapped to toggle enabled? Not sure what that means.");
                                            },
                                        }
                                        
                                    },
                                    _ => {
                                        match pad.mapping.get(&JDCId::AxisAsButton(ev.number(), ev.value())) {
                                            Some((_, target)) => {
                                                match target {
                                                    Target::Key(k) => {
                                                        let code = k.uinput_key().code();
                                                        if let Err(e) = uinput_dev.emit(&[
                                                            InputEvent::new(EventType::KEY, code, 1),
                                                            InputEvent::new(EventType::KEY, code, 0),
                                                        ]){
                                                            eprintln!("Error sending event: {}", e);
                                                        }
                                                    },
                                                    Target::Axis(a) => {
                                                        eprintln!("Warning: Unable to map this button to its axis target because the device models the button as an axis. Target event dropped: {:?}\nFor an explanation of why this happens, see the github issue here: https://github.com/lexbailey/joy2uinput/issues/2", a);
                                                    },
                                                    Target::ToggleEnabled() => {
                                                        enabled = !enabled;
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
                        eprintln!("Error sending event: {}", e);
                    }
                    if axis_speeds.lock().unwrap().values().all(|&a|a==0){
                        *poll_axis.lock().unwrap() = false;
                    }
                }
                Ev::Listen() => {
                    listening = true;
                }
            }
            _ => {break;}
        }
    }
    Ok(())
}
