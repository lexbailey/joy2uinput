/*

What does this file do?

    This is the file with all the datatypes that are used internally in joy2uinput
    These are also used somewhat by joy2u-mapgen.
    This includes all of the parser logic for reading from config files, and also
    the generation logic for writing config files.

*/

use std::str::FromStr;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ffi::OsString;

fn parse_args(s: &str, n: usize) -> Result<Vec<&str>, String> {
    if s.len() < 2{
        return Err(format!("Malformed arguments: {}.", s));
    }
    if !s.starts_with('(') || !s.ends_with(')') {
        return Err(format!("Malformed arguments: {}.", s));
    }
    let inner = &s[1..s.len()-1];
    let args: Vec<_> = inner.split(',').collect();
    if args.len() != n{
        return Err(format!("Expected {} args, found {}", n, args.len()));
    }
    Ok(args)
}

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub enum Button{
    Up(),
    Down(),
    Left(),
    Right(),

    Start(),
    Select(),

    A(),
    B(),
    C(),
    D(),

    W(),
    X(),
    Y(),
    Z(),

    LShoulder(),
    RShoulder(),

    LTrigger(),
    RTrigger(),

    Menu(),
    Home(),
    
    LStick(),
    RStick(),

    Plus(),
    Minus(),

    Custom(u128),
}

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub enum Axis{
    LeftX(),
    LeftY(),
    LeftZ(),

    RightX(),
    RightY(),
    RightZ(),

    Throttle(),
    Brake(),

    ScrollX(),
    ScrollY(),
    ScrollZ(),

    Roll(),
    Pitch(),
    Yaw(),
	Custom(u128),
}

#[derive(Debug,Clone,PartialEq,Eq,Hash)]
pub enum JoyInput{
    Button(Button),
    Axis(Axis),
}

impl Display for Button{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self{
            Button::Custom(n) => {
                write!(f, "custom_button({})", n)
            },
            _ => {
                let s = format!("{:?}", self);
                f.write_str(&s.to_lowercase())
            },
        }
    } 
}

impl Display for Axis{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self{
            Axis::Custom(n) => {
                write!(f, "custom_axis({})", n)
            },
            _ => {
                let s = format!("{:?}", self);
                f.write_str(&s.to_lowercase())
            },
        }
    } 
}

impl Display for JoyInput{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self{
            JoyInput::Button(b) => write!(f, "{}", b),
            JoyInput::Axis(a) => write!(f, "{}", a),
        }
    } 
}

impl FromStr for Button{
    type Err = String;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> { 
        let l = s.trim().to_lowercase();
        match l.as_ref(){
            "up" => Ok(Button::Up()),
            "down" => Ok(Button::Down()),
            "left" => Ok(Button::Left()),
            "right" => Ok(Button::Right()),
            "start" => Ok(Button::Start()),
            "select" => Ok(Button::Select()),
            "a" => Ok(Button::A()),
            "b" => Ok(Button::B()),
            "c" => Ok(Button::C()),
            "d" => Ok(Button::D()),
            "w" => Ok(Button::W()),
            "x" => Ok(Button::X()),
            "y" => Ok(Button::Y()),
            "z" => Ok(Button::Z()),
            "lshoulder" => Ok(Button::LShoulder()),
            "rshoulder" => Ok(Button::RShoulder()),
            "ltrigger" => Ok(Button::LTrigger()),
            "rtrigger" => Ok(Button::RTrigger()),
            "menu" => Ok(Button::Menu()),
            "home" => Ok(Button::Home()),
            "lstick" => Ok(Button::LStick()),
            "rstick" => Ok(Button::RStick()),
            "plus" => Ok(Button::Plus()),
            "minus" => Ok(Button::Minus()),
            _ => {
                if l.starts_with("custom_button"){
                    let args = l[13..].trim();
                    if args.starts_with("(") && args.ends_with(")"){
                        let n: Result<u128,_> = args[1..args.len()-1].parse();
                        if let Ok(n) = n{
                            return Ok(Button::Custom(n));
                        }
                    }
                	return Err(format!("Invalid custom button specifier: {}. Please use 'custom_button(n)' for some natural number 'n'", s))
                }
                Err(format!("No such button: {}", s))
            },
        }
    }
}

impl FromStr for Axis{
    type Err = String;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> { 
        let l = s.trim().to_lowercase();
        match l.as_ref(){
			"leftx" =>    Ok(Axis::LeftX()    ),
			"lefty" =>    Ok(Axis::LeftY()    ),
			"leftz" =>    Ok(Axis::LeftZ()    ),
			"rightx" =>   Ok(Axis::RightX()   ),
			"righty" =>   Ok(Axis::RightY()   ),
			"rightz" =>   Ok(Axis::RightZ()   ),
			"throttle" => Ok(Axis::Throttle() ),
			"brake" =>    Ok(Axis::Brake()    ),
			"scrollx" =>  Ok(Axis::ScrollX()  ),
			"scrolly" =>  Ok(Axis::ScrollY()  ),
			"scrollz" =>  Ok(Axis::ScrollZ()  ),
			"roll" =>     Ok(Axis::Roll()     ),
			"pitch" =>    Ok(Axis::Pitch()    ),
			"yaw" =>      Ok(Axis::Yaw()      ),

            _ => {
                if l.starts_with("custom_axis"){
                    let args = l[11..].trim();
                    if args.starts_with("(") && args.ends_with(")"){
                        let n: Result<u128,_> = args[1..args.len()-1].parse();
                        if let Ok(n) = n{
                            return Ok(Axis::Custom(n));
                        }
                    }
                	return Err(format!("Invalid custom axis specifier: {}. Please use 'custom_axis(n)' for some natural number 'n'", s))
                }
                Err(format!("No such axis: {}", s))
            },
        }
    }
}

impl FromStr for JoyInput{
    type Err = String;

    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> { 
        let btn: Result<Button,_> = s.parse();
        let axis: Result<Axis,_> = s.parse();
        match (btn, axis) {
            (Ok(_), Ok(_)) => { Err(format!("Internal error, {} is an ambiguous input specifier.", s)) },
            (Err(_), Err(_)) => { Err(format!("Unrecognised input specifier: {}. This is not a valid button, or a valid axis.", s)) },
            (Ok(a), Err(_)) => { Ok(JoyInput::Button(a)) },
            (Err(_), Ok(a)) => { Ok(JoyInput::Axis(a)) },
        }
    }
}

#[derive(Debug,Clone)]
pub enum KeyTarget{
    Up(),
    Down(),
    Left(),
    Right(),
    Escape(),
    Return(),
    AlphaNum(char),
    Numpad(u8),
    Space(),
    F(u8),
    PageUp(),
    PageDown(),
    Home(),
    End(),
    Delete(),
    Tab(),
    LCtrl(),
    RCtrl(),
    LShift(),
    RShift(),
    LSuper(),
    RSuper(),
    LAlt(),
    RAlt(),
    Menu(),
    VolUp(),
    VolDown(),
    // not strictly keys, but they look the same to uinput, so this is fine
    MouseButtonLeft(),
    MouseButtonRight(),
    MouseButtonMiddle(),
    MouseButtonSide(),
    MouseButtonExtra(),
    MouseButtonForward(),
    MouseButtonBack(),
}

#[derive(Debug,Clone)]
pub enum AxisTarget{
    MouseX(f32),
    MouseY(f32),
    ScrollX(f32),
    ScrollY(f32),
    PageUpDown(f32),
    LeftRight(f32),
    UpDown(f32),
    VolUpDown(f32),
}

#[derive(Debug,Clone)]
pub enum Target{
    Key(KeyTarget),
    Axis(AxisTarget),
}

impl AxisTarget{
    pub fn uinput_keys(&self) -> Vec<evdev::Key> {
        let mut keys = Vec::new();
        match self{
            AxisTarget::PageUpDown(_) => {
                keys.push(evdev::Key::KEY_PAGEUP);
                keys.push(evdev::Key::KEY_PAGEDOWN);
            },
            AxisTarget::LeftRight(_) => {
                keys.push(evdev::Key::KEY_LEFT);
                keys.push(evdev::Key::KEY_RIGHT);
            },
            AxisTarget::UpDown(_) => {
                keys.push(evdev::Key::KEY_UP);
                keys.push(evdev::Key::KEY_DOWN);
            },
            AxisTarget::VolUpDown(_) => {
                keys.push(evdev::Key::KEY_VOLUMEUP);
                keys.push(evdev::Key::KEY_VOLUMEDOWN);
            },
            AxisTarget::MouseX(_) => {},
            AxisTarget::MouseY(_) => {},
            AxisTarget::ScrollX(_) => {},
            AxisTarget::ScrollY(_) => {},
        }
        keys
    }

    pub fn uinput_axis(&self) -> Option<evdev::RelativeAxisType> {
        match self{
            AxisTarget::PageUpDown(_) => None,
            AxisTarget::LeftRight(_) => None,
            AxisTarget::UpDown(_) => None,
            AxisTarget::VolUpDown(_) => None,
            AxisTarget::MouseX(_) => {
                Some(evdev::RelativeAxisType::REL_X)
            },
            AxisTarget::MouseY(_) => {
                Some(evdev::RelativeAxisType::REL_Y)
            },
            AxisTarget::ScrollX(_) => {
                Some(evdev::RelativeAxisType::REL_HWHEEL)
            },
            AxisTarget::ScrollY(_) => {
                Some(evdev::RelativeAxisType::REL_WHEEL)
            },
        }
    }

    pub fn multiplier(&self) -> f32{
        match self{
            AxisTarget::PageUpDown(m) => *m,
            AxisTarget::LeftRight(m) => *m,
            AxisTarget::UpDown(m) => *m,
            AxisTarget::VolUpDown(m) => *m,
            AxisTarget::MouseX(m) => *m,
            AxisTarget::MouseY(m) => *m,
            AxisTarget::ScrollX(m) => *m,
            AxisTarget::ScrollY(m) => *m,
        }

    }
}

impl KeyTarget{
    pub fn uinput_key(&self) -> evdev::Key{
        match self{
            KeyTarget::Up() => evdev::Key::KEY_UP,
            KeyTarget::Down() => evdev::Key::KEY_DOWN,
            KeyTarget::Left() => evdev::Key::KEY_LEFT,
            KeyTarget::Right() => evdev::Key::KEY_RIGHT,
            KeyTarget::Escape() => evdev::Key::KEY_ESC,
            KeyTarget::Return() => evdev::Key::KEY_ENTER,
            KeyTarget::Space() => evdev::Key::KEY_SPACE,
            KeyTarget::PageUp() => evdev::Key::KEY_PAGEUP,
            KeyTarget::PageDown() => evdev::Key::KEY_PAGEDOWN,
            KeyTarget::Home() => evdev::Key::KEY_HOME,
            KeyTarget::End() => evdev::Key::KEY_END,
            KeyTarget::Delete() => evdev::Key::KEY_DELETE,
            KeyTarget::Tab() => evdev::Key::KEY_TAB,
            KeyTarget::LCtrl() => evdev::Key::KEY_LEFTCTRL,
            KeyTarget::RCtrl() => evdev::Key::KEY_RIGHTCTRL,
            KeyTarget::LShift() => evdev::Key::KEY_LEFTSHIFT,
            KeyTarget::RShift() => evdev::Key::KEY_RIGHTSHIFT,
            KeyTarget::LSuper() => evdev::Key::KEY_LEFTMETA,
            KeyTarget::RSuper() => evdev::Key::KEY_RIGHTMETA,
            KeyTarget::LAlt() => evdev::Key::KEY_LEFTALT,
            KeyTarget::RAlt() => evdev::Key::KEY_RIGHTALT,
            KeyTarget::Menu() => evdev::Key::KEY_MENU,
            KeyTarget::VolUp() => evdev::Key::KEY_VOLUMEUP,
            KeyTarget::VolDown() => evdev::Key::KEY_VOLUMEDOWN,
            KeyTarget::MouseButtonLeft() => evdev::Key::BTN_LEFT,
            KeyTarget::MouseButtonRight() => evdev::Key::BTN_RIGHT,
            KeyTarget::MouseButtonMiddle() => evdev::Key::BTN_MIDDLE,
            KeyTarget::MouseButtonSide() => evdev::Key::BTN_SIDE,
            KeyTarget::MouseButtonExtra() => evdev::Key::BTN_EXTRA,
            KeyTarget::MouseButtonBack() => evdev::Key::BTN_BACK,
            KeyTarget::MouseButtonForward() => evdev::Key::BTN_FORWARD,
            KeyTarget::F(n) => match n{
                1 => evdev::Key::KEY_F1,
                2 => evdev::Key::KEY_F2,
                3 => evdev::Key::KEY_F3,
                4 => evdev::Key::KEY_F4,
                5 => evdev::Key::KEY_F5,
                6 => evdev::Key::KEY_F6,
                7 => evdev::Key::KEY_F7,
                8 => evdev::Key::KEY_F8,
                9 => evdev::Key::KEY_F9,
                10 => evdev::Key::KEY_F10,
                11 => evdev::Key::KEY_F11,
                12 => evdev::Key::KEY_F12,
                13 => evdev::Key::KEY_F13,
                14 => evdev::Key::KEY_F14,
                15 => evdev::Key::KEY_F15,
                16 => evdev::Key::KEY_F16,
                17 => evdev::Key::KEY_F17,
                18 => evdev::Key::KEY_F18,
                19 => evdev::Key::KEY_F19,
                20 => evdev::Key::KEY_F20,
                21 => evdev::Key::KEY_F21,
                22 => evdev::Key::KEY_F22,
                23 => evdev::Key::KEY_F23,
                24 => evdev::Key::KEY_F24,
                _ => evdev::Key::KEY_RESERVED,
            },
            KeyTarget::Numpad(n) => match n {
                0 => evdev::Key::KEY_NUMERIC_0,
                1 => evdev::Key::KEY_NUMERIC_1,
                2 => evdev::Key::KEY_NUMERIC_2,
                3 => evdev::Key::KEY_NUMERIC_3,
                4 => evdev::Key::KEY_NUMERIC_4,
                5 => evdev::Key::KEY_NUMERIC_5,
                6 => evdev::Key::KEY_NUMERIC_6,
                7 => evdev::Key::KEY_NUMERIC_7,
                8 => evdev::Key::KEY_NUMERIC_8,
                9 => evdev::Key::KEY_NUMERIC_9,
                _ => evdev::Key::KEY_RESERVED,
            },
            KeyTarget::AlphaNum(c) => match c {
                'a' => evdev::Key::KEY_A,
                'b' => evdev::Key::KEY_B,
                'c' => evdev::Key::KEY_C,
                'd' => evdev::Key::KEY_D,
                'e' => evdev::Key::KEY_E,
                'f' => evdev::Key::KEY_F,
                'g' => evdev::Key::KEY_G,
                'h' => evdev::Key::KEY_H,
                'i' => evdev::Key::KEY_I,
                'j' => evdev::Key::KEY_J,
                'k' => evdev::Key::KEY_K,
                'l' => evdev::Key::KEY_L,
                'm' => evdev::Key::KEY_M,
                'n' => evdev::Key::KEY_N,
                'o' => evdev::Key::KEY_O,
                'p' => evdev::Key::KEY_P,
                'q' => evdev::Key::KEY_Q,
                'r' => evdev::Key::KEY_R,
                's' => evdev::Key::KEY_S,
                't' => evdev::Key::KEY_T,
                'u' => evdev::Key::KEY_U,
                'v' => evdev::Key::KEY_V,
                'w' => evdev::Key::KEY_W,
                'x' => evdev::Key::KEY_X,
                'y' => evdev::Key::KEY_Y,
                'z' => evdev::Key::KEY_Z,
                '1' => evdev::Key::KEY_1,
                '2' => evdev::Key::KEY_2,
                '3' => evdev::Key::KEY_3,
                '4' => evdev::Key::KEY_4,
                '5' => evdev::Key::KEY_5,
                '6' => evdev::Key::KEY_6,
                '7' => evdev::Key::KEY_7,
                '0' => evdev::Key::KEY_0,
                '-' => evdev::Key::KEY_MINUS,
                '=' => evdev::Key::KEY_EQUAL,
                //'`' => evdev::Key::KEY_, // TODO what is this called?
                '[' => evdev::Key::KEY_LEFTBRACE,
                ']' => evdev::Key::KEY_RIGHTBRACE,
                ';' => evdev::Key::KEY_SEMICOLON,
                '\'' => evdev::Key::KEY_APOSTROPHE,
                // '#' => evdev::Key::KEY_, // TODO what is this called?
                ',' => evdev::Key::KEY_COMMA,
                '.' => evdev::Key::KEY_DOT,
                '/' => evdev::Key::KEY_SLASH,
                '\\' => evdev::Key::KEY_BACKSLASH, 
                _ => evdev::Key::KEY_RESERVED,
            },
        }
    }
}

impl FromStr for KeyTarget{
    type Err = String;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let l = s.to_lowercase();
        if !l.starts_with("key"){
            if !l.starts_with("mousebutton"){
                Err(format!("Invalid key target specifier: {}", s))
            }
            else{
                let args = parse_args(&l[11..], 1);
                match args{
                    Err(e) => {Err(format!("Malformed arguments to key target specifier: {}. {}", s, e))},
                    Ok (args) => {
                        match args[0] {
                            "left" => Ok(KeyTarget::MouseButtonLeft()),
                            "right" => Ok(KeyTarget::MouseButtonRight()),
                            "middle" => Ok(KeyTarget::MouseButtonMiddle()),
                            "side" => Ok(KeyTarget::MouseButtonSide()),
                            "extra" => Ok(KeyTarget::MouseButtonExtra()),
                            "forward" => Ok(KeyTarget::MouseButtonForward()),
                            "back" => Ok(KeyTarget::MouseButtonBack()),
                            s => {
                                Err(format!("Malformed arguments to mouse button target specifier: {}", s))
                            },
                        }
                    }
                }
            }
        }
        else{
            let args = parse_args(&l[3..], 1);
            match args{
                Err(e) => {Err(format!("Malformed arguments to key target specifier: {}. {}", s, e))},
                Ok(args) => {
                    match args[0] {
                        "up" => Ok(KeyTarget::Up()),
                        "down" => Ok(KeyTarget::Down()),
                        "left" => Ok(KeyTarget::Left()),
                        "right" => Ok(KeyTarget::Right()),
                        "escape" | "esc" => Ok(KeyTarget::Escape()),
                        "return" | "enter" => Ok(KeyTarget::Return()),
                        "space" | "spacebar" => Ok(KeyTarget::Space()),
                        "pageup" | "pgup" => Ok(KeyTarget::PageUp()),
                        "pagedown" | "pgdn" => Ok(KeyTarget::PageDown()),
                        "home" => Ok(KeyTarget::Home()),
                        "end" => Ok(KeyTarget::End()),
                        "delete" => Ok(KeyTarget::Delete()),
                        "tab" => Ok(KeyTarget::Tab()),
                        "lctrl" | "lcontrol" => Ok(KeyTarget::LCtrl()),
                        "rctrl" | "rcontrol" => Ok(KeyTarget::RCtrl()),
                        "lshift" => Ok(KeyTarget::LShift()),
                        "rshift" => Ok(KeyTarget::RShift()),
                        "lsuper" => Ok(KeyTarget::LSuper()),
                        "rsuper" => Ok(KeyTarget::RSuper()),
                        "lalt" => Ok(KeyTarget::LAlt()),
                        "ralt" => Ok(KeyTarget::RAlt()),
                        "menu" => Ok(KeyTarget::Menu()),
                        "volup" | "volumeup" => Ok(KeyTarget::VolUp()),
                        "voldown" | "volumedown" => Ok(KeyTarget::VolDown()),
                        a => {
                            if a.len() == 1{
                                Ok(KeyTarget::AlphaNum(a.chars().next().unwrap()))
                            }
                            else{
                                if a.starts_with("f"){
                                    let num = a[1..].parse::<u8>();
                                    match num{
                                        Ok(num) => Ok(KeyTarget::F(num)),
                                        Err(e) => Err(format!("Invalid key target specifier: {}. {}", s, e)),
                                    }
                                }
                                else if a.starts_with("numpad"){
                                    let num = a[6..].parse::<u8>();
                                    match num{
                                        Ok(num) => Ok(KeyTarget::Numpad(num)),
                                        Err(e) => Err(format!("Invalid key target specifier: {}. {}", s, e)),
                                    }
                                }
                                else{
                                    Err(format!("Invalid key target specifier: {}", s))
                                }
                            }
                        }
                    }
                },
            }
        }
    }
}

impl FromStr for AxisTarget{
    type Err = String;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let l = s.to_lowercase();
        if !l.starts_with("axis"){
            return Err(format!("Invalid axis target specifier: {}", s));
        }
        else{
            let args = parse_args(&l[4..], 2);
            match args{
                Err(e) => {Err(format!("Malformed arguments to key target specifier: {}. {}", s, e))},
                Ok (args) => {
                    let mult = args[1].trim().parse::<f32>().or_else(|_|{args[1].trim().parse::<i32>().map(|a|a as f32)});
                    match mult{
                        Err(_) => {Err(format!("Malformed arguments to key target specifier: {}. Argument 2 should be a float", s))},
                        Ok(mult) => {
                            match args[0] {
                                "mousex" => Ok(AxisTarget::MouseX(mult)),
                                "mousey" => Ok(AxisTarget::MouseY(mult)),
                                "scrollx" => Ok(AxisTarget::ScrollX(mult)),
                                "scrolly" => Ok(AxisTarget::ScrollY(mult)),
                                "pageupdown" => Ok(AxisTarget::PageUpDown(mult)),
                                "leftright" => Ok(AxisTarget::LeftRight(mult)),
                                "updown" => Ok(AxisTarget::UpDown(mult)),
                                "volupdown" => Ok(AxisTarget::VolUpDown(mult)),
                                _ => Err(format!("Invalid axis target specifier: {}", s)),
                            }
                        }
                    }
                },
            }
        }
    }
}

impl FromStr for Target{
    type Err = String;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let l = s.to_lowercase();
        if l.starts_with("key"){
            return Ok(Target::Key(s.parse()?));
        }
        if l.starts_with("mousebutton"){
            return Ok(Target::Key(s.parse()?));
        }
        if l.starts_with("axis"){
            return Ok(Target::Axis(s.parse()?));
        }
        Err(format!("Unrecognised uinput target specifier: {}", s))
    }
}

#[derive(Debug,PartialEq,Eq,Hash,Clone)]
pub enum JDEv{
    Button(u8),
    AxisAsButton(u8, i16),
    Axis(u8, i16, i16),
}

impl Display for JDEv{
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        match self {
		    JDEv::Button(b) => write!(f, "button({})", b),
		    JDEv::AxisAsButton(a, val) => write!(f, "axis_as_button({},{})", a, val),
		    JDEv::Axis(a,min,max) => write!(f, "axis({},{},{})", a,min,max),
        }
	}
}

fn string_err<T,E>(r: Result<T,E>) -> Result<T,String> where E: Display {
    match r{
        Ok(a) => Ok(a),
        Err(e) => Err(format!("{}", e)),
    }
}

impl FromStr for JDEv{
    type Err = String;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
        let l = s.trim().to_lowercase();
        if l.starts_with("button"){
            let args = l[6..].trim();
            return match (||->Result<JDEv, String>{
                let args = parse_args(args, 1)?;
                return Ok(JDEv::Button(string_err(args[0].parse())?));
            })(){
                Err(a) => Err(format!("Unable to parse joydev event 'button': {}", a)),
                Ok(a) => Ok(a),
            };
        }
        if l.starts_with("axis_as_button"){
            let args = l[14..].trim();
            return match (||->Result<JDEv, String>{
                let args = parse_args(args, 2)?;
                return Ok(JDEv::AxisAsButton(string_err(args[0].parse())?,string_err(args[1].parse())?));
            })(){
                Err(a) => Err(format!("Unable to parse joydev event 'axis_as_button': {}", a)),
                Ok(a) => Ok(a),
            };
        }
        if l.starts_with("axis"){
            let args = l[4..].trim();
            return match (||->Result<JDEv, String>{
                let args = parse_args(args, 3)?;
                return Ok(JDEv::Axis(string_err(args[0].parse())?,string_err(args[1].parse())?,string_err(args[2].parse())?));
            })(){
                Err(a) => Err(format!("Unable to parse joydev event 'axis': {}", a)),
                Ok(a) => Ok(a),
            };
        }
        Err(format!("Unrecognised joydev event type: {}.", s))
	}
}

#[derive(Debug)]
pub struct Mapping{
    pub from: JDEv,
    pub to: JoyInput,
}

impl Display for Mapping{
	fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
		write!(f, "{} = {}", self.from, self.to)
	}
}

impl FromStr for Mapping{
    type Err = String;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
		// syntax is: "JDEv = JoyInput"
		let sides: Vec<_> = s.split("=").collect();
		if sides.len() != 2{
			return Err(format!("Invalid mapping. Expected exactly one '=' character. '<from> = <to>'"));
		}
		let left = sides[0].trim();
		let right = sides[1].trim();
        let jdev = left.parse::<JDEv>();
        let joyinput = right.parse::<JoyInput>();
        // TODO: more helpful error messages with column numbers?
        match (jdev, joyinput) {
            (Ok(jd), Ok(ji)) => Ok(Mapping{from: jd, to: ji}),
            (Ok(_), Err(ji)) => Err(ji),
            (Err(jd), Ok(_)) => Err(jd),
            (Err(jd), Err(ji)) => Err(format!("{}, also {}", jd, ji)),
        }
	}
}

#[derive(Debug)]
pub struct TargetMapping{
    pub from: JoyInput,
    pub to: Target,
}

impl FromStr for TargetMapping{
    type Err = String;
    fn from_str(s: &str) -> Result<Self, <Self as FromStr>::Err> {
		// syntax is: "JDEv = JoyInput"
		let sides: Vec<_> = s.split("=").collect();
		if sides.len() != 2{
			return Err(format!("Invalid mapping. Expected exactly one '=' character. '<from> = <to>'"));
		}
		let left = sides[0].trim();
		let right = sides[1].trim();
        let joyinput = left.parse::<JoyInput>();
        let target = right.parse::<Target>();
        // TODO: more helpful error messages with column numbers?
        match (joyinput, target) {
            (Ok(ji), Ok(targ)) => Ok(TargetMapping{from: ji, to: targ}),
            (Ok(_), Err(targ)) => Err(targ),
            (Err(ji), Ok(_)) => Err(ji),
            (Err(ji), Err(targ)) => Err(format!("{}, also {}", ji, targ)),
        }
	}
}
pub fn jpname_to_filename(jp: &str) -> OsString{
    let mut s = OsString::from(jp
        .replace("_", "___")
        .replace("/", "__-")
        .replace("\\", "_-_")
    );
    s.push(".j2umap");
    s
}
