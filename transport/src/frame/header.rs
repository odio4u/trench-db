use crate::{errors::TransportError, frame};


pub const FLAG_FIN:      u16 = 0b0000_0000_0000_0001; // bit 0
pub const FLAG_ACK:      u16 = 0b0000_0000_0000_0010; // bit 1
pub const FLAG_CONTROL:  u16 = 0b0000_0000_0000_0100; // bit 2


// The magic number we write into every frame we send, to help detect framing errors. TRNC
pub const FRAME_MAGIC: [u8; 4] = [0x54, 0x52, 0x4E, 0x43];
 
// The version number we write into every frame we send.
pub const CURRENT_VERSION: u8 = 1;
 
// The oldest version we are willing to speak with a peer.
pub const MIN_SUPPORTED_VERSION: u8 = 1;
 
// Maximum allowed payload size: 16 MiB.
// 16 * 1024 * 1024 = 16,777,216 bytes.
// We reject frames claiming a larger payload before reading any payload bytes.
// This prevents a malicious peer from making us allocate gigabytes of memory.
pub const MAX_FRAME_SIZE: usize = 16 * 1024 * 1024;
 
// The header is always exactly 16 bytes:
//   magic[4] + length[4] + stream_id[4] + flags[2] + frame_type[1] + version[1]
pub const HEADER_SIZE: usize = 16;



#[derive(Debug, Clone)]
pub struct Header {
    pub magic: [u8; 4], // "TRNS"
    pub version: u8,
    pub flags: u16,
    pub stream_id: u32,
    pub payload_length: u16,
    pub frame_type: frame::frame::Frametype,
}


impl Header {

    pub fn is_fin(&self) -> bool {
        self.flags & FLAG_FIN != 0
    }

    pub fn is_ack(&self) -> bool {
        self.flags & FLAG_ACK != 0
    }

    pub fn is_control(&self) -> bool {
        self.flags & FLAG_CONTROL != 0
    }

    pub fn validate(&self) -> Result<(), TransportError> {
        if self.magic != FRAME_MAGIC {
            return Err(TransportError::InvalidMagic);
        }
        if self.version < MIN_SUPPORTED_VERSION || self.version > CURRENT_VERSION {
            return Err(TransportError::InvalidVersion { got: self.version, min: MIN_SUPPORTED_VERSION, max: CURRENT_VERSION });
        }
        if self.payload_length as usize > MAX_FRAME_SIZE {
            return Err(TransportError::FrameTooLarge { size: self.payload_length as usize, max: MAX_FRAME_SIZE });
        }
        if self.frame_type as u8 > frame::frame::Frametype::Hello as u8 {
            return Err(TransportError::InvalidFrame(format!("Unknown frame type: {}", self.frame_type as u8)));
        }
        if self.flags & !(FLAG_FIN | FLAG_ACK | FLAG_CONTROL) != 0 {
            return Err(TransportError::InvalidFrame(format!("Unknown flags set: {:016b}", self.flags)));
        }
        Ok(())
    }

    pub fn new(frame_type: frame::frame::Frametype, flags: u16, stream_id: u32, payload_length: u16) -> Self {
        Header {
            magic: FRAME_MAGIC,
            version: CURRENT_VERSION,
            flags,
            stream_id,
            payload_length,
            frame_type,
        }
    }

}

