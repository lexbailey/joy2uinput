use std::str::FromStr;
use std::fmt::Display;
use std::fmt::Formatter;
use std::ffi::OsString;

#[derive(Debug,Clone)]
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

    Unmappable(),
}

#[derive(Debug,Clone)]
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

#[derive(Debug,Clone)]
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

#[derive(Debug)]
pub enum ButtonTarget{
    Up(),
    Down(),
    Left(),
    Right(),
    Escape(),
    Return(),
    AlphaNum(char),
    Space(),
}

#[derive(Debug)]
pub enum AxisTarget{
    MouseX(),
    MouseY(),
    ScrollX(),
    ScrollY(),
    PageUpDown(),
    LeftRight(),
    UpDown(),
}

#[derive(Debug)]
pub enum Target{
    Button(ButtonTarget),
    Axis(AxisTarget),
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

pub fn jpname_to_filename(jp: &str) -> OsString{
    let mut s = OsString::from(jp
        .replace("_", "___")
        .replace("/", "__-")
        .replace("\\", "_-_")
    );
    s.push(".j2umap");
    s
}
