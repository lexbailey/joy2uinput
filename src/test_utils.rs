#![allow(non_snake_case)]
use std::io::{Read,ErrorKind};

pub fn new_virtual_joypad(name: &str) -> evdev::uinput::VirtualDevice {
    let mut keys = evdev::AttributeSet::new();
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
pub enum TestEv{
    Timeout(),
    Line(String),
}

pub fn spawn_main<T>(wrapped_main: T) -> (std::thread::JoinHandle<()>, std::sync::mpsc::Receiver<TestEv>, std::thread::JoinHandle<()>) where T: FnOnce(std::os::unix::net::UnixStream) -> () + std::marker::Send + 'static{
    let (send, recv) = std::sync::mpsc::channel();
    let send1 = send.clone();

    let timeout_join = std::thread::spawn(move||{
        std::thread::sleep(std::time::Duration::from_secs(10));
        send1.send(TestEv::Timeout()).unwrap();
    });

    let send2 = send.clone();
    let jh = std::thread::spawn(move||{
        let (mut stdout, t_stdout) = std::os::unix::net::UnixStream::pair().unwrap();
        let _main_join = std::thread::spawn(move||{wrapped_main(t_stdout);});

        let mut line = String::new();
        stdout.set_read_timeout(Some(std::time::Duration::from_millis(100))).unwrap();
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
                            send2.send(TestEv::Line(line.clone())).unwrap();
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
    (jh,recv,timeout_join)
}

