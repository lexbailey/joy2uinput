#![allow(non_upper_case_globals)]
use std::path::PathBuf;
use std::fmt::Display;
use std::fmt::Debug;
use inotify::{ Inotify, WatchMask, EventMask };
use std::sync::mpsc::{channel, Sender, Receiver};
use std::ffi::{OsString, OsStr};
use std::collections::HashMap;
use std::thread::JoinHandle;
use std::path::Path;
use std::fs::OpenOptions;
use std::os::fd::AsRawFd;
use std::mem::MaybeUninit;

const conf_dir_env_var: &'static str = "JOY2UINPUT_CONFDIR";

fn get_user_conf_dir() -> (PathBuf, bool){
    if let Ok(d) = std::env::var(conf_dir_env_var){
        return (PathBuf::from(&d), true);
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

enum Ev{
    Joy(joydev::DeviceEvent),
    Key(),
    Connect(OsString),
    Disconnect(OsString),
}

fn hotplug_thread(evs: Sender<Ev>) -> Option<std::thread::JoinHandle<()>> {
    let mut inotify = match (||->std::io::Result<Inotify>{
                let i = Inotify::init()?;
                i.watches().add("/dev/input", WatchMask::CREATE | WatchMask::DELETE | WatchMask::ATTRIB)?;
                Ok(i)
            })() {
        Ok(a) => { Some(a)},
        Err(e) => { eprintln!("Warning: failed to start inotify, hotplugging is unavailable"); None},
    };
	
    if let Some(mut inotify) = inotify{
        Some(std::thread::spawn(move || {
            let mut buffer = [0; 1024];
            loop{
                if let Ok(mut events) = inotify.read_events_blocking(&mut buffer){
                    for event in events{
                        let n = event.name.unwrap();
                        if n.to_string_lossy().starts_with("js"){
                            let mut path = PathBuf::from("/dev/input");
                            path.push(n);
                            match event.mask {
                                EventMask::CREATE => { evs.send(Ev::Connect(path.into())); },
                                EventMask::ATTRIB => { evs.send(Ev::Connect(path.into())); },
                                EventMask::DELETE => { evs.send(Ev::Disconnect(path.into())); },
                                _ => unreachable!()
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
    println!("{}", s.display());
    let fd = OpenOptions::new().read(true).open(s)?;
    let dev = unsafe{joydev::Device::from_raw_fd(fd.as_raw_fd())?};
    let name = dev.identifier().to_string();
    Ok((name.clone(), fd, std::thread::spawn(move ||{
        loop{
            match dev.get_event(){
                Ok(ev) => {
                    evs.send(Ev::Joy(ev));
                },
                _ => {break;}
            }
        }
        println!("Pad thread terminated: {}", name);
    })))
}

fn main() -> Result<(),Fatal> {
    println!("joy2u_mapgen - generate a mapping file for joy2uinput");
    println!("");

    let (user_conf_dir, is_from_env) = get_user_conf_dir();

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



    let (send, recv) = std::sync::mpsc::channel::<Ev>();
    let hp_thred = hotplug_thread(send.clone());
    // enumerate already connected joypads
    let mut n_pads: usize = 0;
    match std::fs::read_dir("/dev/input"){
        Err(_) => return Err(Fatal::Msg("Unable to read from /dev/input".to_string())),
        Ok(d) => {
            for f in d{
                if let Ok(j) = f{
                    let n = j.path();
                    if n.to_string_lossy().starts_with("/dev/input/js"){
                        send.send(Ev::Connect(n.into()));
                        n_pads += 1;
                    }
                }
            }
        }
    }

    let mut pads = HashMap::new();
    
    println!("");
    println!("To start generating a config, press any button on the joypad to configure. ({} devices currently connected)", n_pads);

    loop{
        match recv.recv(){
            Ok(msg) => match msg {
                Ev::Connect(s) => {
                    println!("Device connected: {}", &s.to_string_lossy());
                    if !pads.contains_key(&s){
                        let t = pad_thread(send.clone(), &Path::new(&s));
                        match t{
                            Ok(t) => {pads.insert(s,t);}
                            Err(e) => {eprintln!("Error connecting to joypad {:?}", e);}
                        }
                        println!("mapping: {:?}", pads);
                    }
                },
                Ev::Disconnect(s) => {
                    println!("Device disconnected: {}", &s.to_string_lossy());
                    if let Some((n, fd, join)) = pads.remove(&s){
                        join.join();
                    }
                },
                Ev::Key() => {},
                Ev::Joy(ev) => {println!("Input from joypad: {:?}", ev)},
            }
            _ => {break;}
        }
    }

    Ok(())
}
