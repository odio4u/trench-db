use crate::frame;


#[derive(Debug, Clone)]
pub struct  Frame {
    pub header: frame::header::Header,
    pub payload: Vec<u8>,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
#[repr(u8)]
pub enum Frametype {
    Open     = 1,
    Data     = 2,
    Close    = 3,
    Reset    = 4,
    Ping     = 5,
    Pong     = 6,
    Window   = 7,
    Error    = 8,
    Settings = 9,
    Hello    = 10,
    Welcome  = 11,
}

impl Frametype {
    pub fn from_u8(value: u8) -> Option<Self> {
        match value {
            1  => Some(Frametype::Open),
            2  => Some(Frametype::Data),
            3  => Some(Frametype::Close),
            4  => Some(Frametype::Reset),
            5  => Some(Frametype::Ping),
            6  => Some(Frametype::Pong),
            7  => Some(Frametype::Window),
            8  => Some(Frametype::Error),
            9  => Some(Frametype::Settings),
            10 => Some(Frametype::Hello),
            11 => Some(Frametype::Welcome),
            _  => None,
        }
    }
}

impl Frame {
    pub fn new(frame_type: frame::frame::Frametype, flags: u16, stream_id: u32, payload: Vec<u8>) -> Self {
        let header = frame::header::Header::new(frame_type, flags, stream_id, payload.len() as u32);
        Frame { header, payload }
    }
    
}