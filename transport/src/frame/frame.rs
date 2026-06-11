use crate::frame;


#[derive(Debug, Clone)]
pub struct  Frame {
    pub header: frame::header::Header,
    pub payload: Vec<u8>,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Frametype {
    Open = 0,
    Data = 1,
    Close = 2,
    Reset = 3,
    Ping = 4,
    Pong = 5,
    Window = 6,
    Error = 7,
    Settings = 8,
    Hello = 9,
}

impl Frametype {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            0 => Some(Frametype::Open),
            1 => Some(Frametype::Data),
            2 => Some(Frametype::Close),
            3 => Some(Frametype::Reset),
            4 => Some(Frametype::Ping),
            5 => Some(Frametype::Pong),
            6 => Some(Frametype::Window),
            7 => Some(Frametype::Error),
            8 => Some(Frametype::Settings),
            9 => Some(Frametype::Hello),
            _ => None,
        }
    }
}

impl Frame {
    pub fn new(header: frame::header::Header, payload: Vec<u8>) -> Self {
        Frame { header, payload }
    }
    
}