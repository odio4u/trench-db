use crate::wal::error::WalError;

/// Encodes a UTF-8 string as a length-prefixed byte sequence.
pub fn encode_string(buf: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    buf.extend_from_slice(bytes);
}

/// Encodes an arbitrary byte slice as a length-prefixed byte sequence.
pub fn encode_bytes(buf: &mut Vec<u8>, value: &[u8]) {
    buf.extend_from_slice(&(value.len() as u32).to_be_bytes());
    buf.extend_from_slice(value);
}

/// Decodes a length-prefixed UTF-8 string starting at `offset`.
///
/// Returns the decoded string and the number of bytes consumed.
pub fn decode_string(payload: &[u8], offset: usize) -> Result<(String, usize), WalError> {
    let len = read_u32(payload, offset)? as usize;
    let start = offset
        .checked_add(4)
        .ok_or_else(|| WalError::InvalidPayload("string offset overflow".to_string()))?;
    let end = start
        .checked_add(len)
        .ok_or_else(|| WalError::InvalidPayload("string length overflow".to_string()))?;
    let bytes = payload
        .get(start..end)
        .ok_or_else(|| WalError::InvalidPayload("string bytes missing".to_string()))?;
    let string = String::from_utf8(bytes.to_vec())
        .map_err(|err| WalError::InvalidPayload(format!("invalid utf8: {}", err)))?;
    Ok((string, 4 + len))
}

/// Decodes a length-prefixed byte slice starting at `offset`.
///
/// Returns the decoded bytes and the number of bytes consumed.
pub fn decode_bytes(payload: &[u8], offset: usize) -> Result<(Vec<u8>, usize), WalError> {
    let len = read_u32(payload, offset)? as usize;
    let start = offset
        .checked_add(4)
        .ok_or_else(|| WalError::InvalidPayload("bytes offset overflow".to_string()))?;
    let end = start
        .checked_add(len)
        .ok_or_else(|| WalError::InvalidPayload("bytes length overflow".to_string()))?;
    let bytes = payload
        .get(start..end)
        .ok_or_else(|| WalError::InvalidPayload("bytes missing".to_string()))?;
    Ok((bytes.to_vec(), 4 + len))
}

/// Reads a big-endian `u32` from `payload` at `offset`.
pub fn read_u32(payload: &[u8], offset: usize) -> Result<u32, WalError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| WalError::InvalidPayload("u32 offset overflow".to_string()))?;
    let bytes = payload
        .get(offset..end)
        .ok_or_else(|| WalError::InvalidPayload("length header missing".to_string()))?;
    Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
}
