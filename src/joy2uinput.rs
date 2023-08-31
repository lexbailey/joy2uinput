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
use std::time::Duration;
mod map_config;
use map_config::{JDEv, Button, Axis, JoyInput, Target};
use joydev::GenericEvent;
use std::fs::File;
use std::rc::Rc;

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

#[derive(Debug,Default,Clone,Copy)]
struct AxisMotion{
    min: i16,
    max: i16,
    n_events: u64,
}

enum Ev{
    Joy(u32, joydev::Event),
    Connect(OsString, u32),
    Disconnect(OsString, u32),
    Listen(),
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
                                        EventMask::DELETE => { evs.send(Ev::Disconnect(path.into(), id)) },
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

fn read_mappings(path: &PathBuf, mappings: &mut HashMap<OsString, HashMap<JDEv, &JoyInput>>){
    // TODO
    eprintln!("Todo: load config from {}", path.display());
}

fn read_config(path: &PathBuf) -> Option<HashMap<JoyInput, Target>>{
    None // TODO
}

enum Fatal{ Msg(String) }

impl Debug for Fatal{
   fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self{
            Fatal::Msg(s) => write!(f, "{}", s),
        }
   } 
}

struct ConnectedPad{
    name: String,
    file: File,
    mapping: Option<Rc<HashMap<JDEv,Target>>>,
    join: JoinHandle<()>,
}

fn main() -> Result<(),Fatal> {

    let mut pads: HashMap<u32,ConnectedPad> = HashMap::new();
    let mut listening = false;
    let mut _wait_thread = None;
    let mut mappings: HashMap<OsString, HashMap<JDEv, &JoyInput>> = HashMap::new();
    let mut expanded_mappings: HashMap<OsString, Rc<HashMap<JDEv, Target>>> = HashMap::new(); // TODO populate this after loading all config

    let mut outmap = None;

    if let Some(user_conf_dir) = get_user_conf_dir(){
        read_mappings(&user_conf_dir, &mut mappings);
        outmap = read_config(&user_conf_dir);
    }

    let default_conf = PathBuf::from("/etc/joy2uinput/");
    if default_conf.is_dir(){
        read_mappings(&default_conf, &mut mappings);
        if outmap.is_none(){
            outmap = read_config(&default_conf);
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

    loop{
        match recv.recv(){
            Ok(msg) => match msg {
                Ev::Connect(s, id) => {
                    listening = false;
                    if !pads.contains_key(&id){
                        let t = pad_thread(send.clone(), &Path::new(&s));
                        match t{
                            Ok((name, file, join)) => {
                                pads.insert(id,ConnectedPad{
                                    name,
                                    file,
                                    mapping: None, // TODO use actual mapping
                                    join,
                                });
                            }
                            Err(e) => {eprintln!("Error connecting to joypad {}, will retry if device file attributes change...", e);}
                        }
                    }
                    _wait_thread = Some(listen_after(send.clone(), 200));
                },
                Ev::Disconnect(s, id) => {
                    let pad = pads.remove(&id);
                    if pad.is_none(){
                        continue;
                    }
                    pad.unwrap().join.join();
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
                                // TODO
                                eprintln!("todo: deal with button");
                            },
                            joydev::EventType::Axis | joydev::EventType::AxisSynthetic => {
                                // TODO
                                eprintln!("todo: deal with axis");
                            },
                        }
                    }
                },
                Ev::Listen() => {
                    listening = true;
                }
            }
            _ => {break;}
        }
    }
    Ok(())
}
