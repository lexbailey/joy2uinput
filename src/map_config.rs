use std::str::FromStr;
use std::fmt::Display;
use std::fmt::Formatter;

#[derive(Debug)]
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

#[derive(Debug)]
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
}

#[derive(Debug)]
pub enum JoyInput{
    Button(Button),
    Axis(Axis),
}

impl Display for Button{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let s = format!("{:?}", self);
        f.write_str(&s.to_lowercase())
    } 
}

impl Display for Axis{
    fn fmt(&self, f: &mut Formatter<'_>) -> Result<(), std::fmt::Error> {
        let s = format!("{:?}", self);
        f.write_str(&s.to_lowercase())
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
    type Err = ();

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
                if l.starts_with("custom"){
                    let args = l[6..].trim();
                    if args.starts_with("(") && args.ends_with(")"){
                        let n: Result<u128,_> = args[1..args.len()-1].parse();
                        if let Ok(n) = n{
                            return Ok(Button::Custom(n));
                        }
                    }
                }
                Err(())
            },
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
