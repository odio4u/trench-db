
use super::{frame};
use super::header::HEADER_SIZE;


pub fn decode(buffer: &[u8]) -> Result<(frame::Frame, usize), crate::errors::TransportError> {
    use crate::frame::header::Header;
    use crate::frame::frame::Frametype;

    if buffer.len() < HEADER_SIZE {
        return Err(crate::errors::TransportError::InvalidFrame(format!("Buffer too small for header: {} bytes", buffer.len())));
    }

    let magic = [buffer[0], buffer[1], buffer[2], buffer[3]];
    let version = buffer[4];
    let flags = u16::from_be_bytes([buffer[5], buffer[6]]);
    let stream_id = u32::from_be_bytes([buffer[7], buffer[8], buffer[9], buffer[10]]);
    let payload_length = u32::from_be_bytes([buffer[11], buffer[12], buffer[13], buffer[14]]);
    let frame_type_u8 = buffer[15];
    
    let frame_type = Frametype::from_u8(frame_type_u8)
        .ok_or_else(|| crate::errors::TransportError::InvalidFrame(format!("Unknown frame type: {}", frame_type_u8)))?;

    let header = Header {
        magic,
        version,
        flags,
        stream_id,
        payload_length,
        frame_type,
    };

    header.validate()?;

    if buffer.len() < HEADER_SIZE + payload_length as usize {
        return Err(crate::errors::TransportError::InvalidFrame(format!("Buffer too small for payload: expected {} bytes, got {}", payload_length, buffer.len() - HEADER_SIZE)));
    }

    let payload = buffer[HEADER_SIZE..HEADER_SIZE + payload_length as usize].to_vec();
    let total_len = HEADER_SIZE + payload_length as usize;

    Ok((frame::Frame { header, payload }, total_len))
}