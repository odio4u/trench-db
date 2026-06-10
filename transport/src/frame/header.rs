


#[derive(Debug)]
pub struct Header {
    pub magic: [u8; 4], // "TRNS"
    pub version: u8,
    pub flags: u8,
    pub stream_id: u32,
    pub payload_length: u16,
}

#[derive(Debug)]
pub struct  Frame {
    pub header: Header,
    pub payload: Vec<u8>,
}

