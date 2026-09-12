use crate::wal::codec::{decode_bytes, decode_string, encode_bytes, encode_string};
use crate::wal::error::WalError;

/// A single record in the write-ahead log.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum WalEntry {
    CreateTable { table: String },
    RemoveTable { table: String },
    Insert { table: String, key: String, value: Vec<u8> },
    Update { table: String, key: String, value: Vec<u8> },
    Delete { table: String, key: String },
}

impl WalEntry {
    /// Serializes the entry into its payload representation.
    pub fn to_bytes(&self) -> Vec<u8> {
        let mut buf = Vec::new();
        match self {
            WalEntry::CreateTable { table } => {
                buf.push(0);
                encode_string(&mut buf, table);
            }
            WalEntry::RemoveTable { table } => {
                buf.push(1);
                encode_string(&mut buf, table);
            }
            WalEntry::Insert { table, key, value } => {
                buf.push(2);
                encode_string(&mut buf, table);
                encode_string(&mut buf, key);
                encode_bytes(&mut buf, value);
            }
            WalEntry::Update { table, key, value } => {
                buf.push(3);
                encode_string(&mut buf, table);
                encode_string(&mut buf, key);
                encode_bytes(&mut buf, value);
            }
            WalEntry::Delete { table, key } => {
                buf.push(4);
                encode_string(&mut buf, table);
                encode_string(&mut buf, key);
            }
        }
        buf
    }

    /// Deserializes an entry from its payload representation.
    pub fn from_bytes(payload: &[u8]) -> Result<Self, WalError> {
        let mut cursor = 0;
        let tag = *payload
            .first()
            .ok_or_else(|| WalError::InvalidPayload("missing tag".to_string()))?;
        cursor += 1;

        let (table, size) = decode_string(payload, cursor)?;
        cursor += size;

        let entry = match tag {
            0 => WalEntry::CreateTable { table },
            1 => WalEntry::RemoveTable { table },
            2 => {
                let (key, key_size) = decode_string(payload, cursor)?;
                cursor += key_size;
                let (value, value_size) = decode_bytes(payload, cursor)?;
                cursor += value_size;
                WalEntry::Insert { table, key, value }
            }
            3 => {
                let (key, key_size) = decode_string(payload, cursor)?;
                cursor += key_size;
                let (value, value_size) = decode_bytes(payload, cursor)?;
                cursor += value_size;
                WalEntry::Update { table, key, value }
            }
            4 => {
                let (key, key_size) = decode_string(payload, cursor)?;
                cursor += key_size;
                WalEntry::Delete { table, key }
            }
            _ => return Err(WalError::InvalidPayload(format!("unknown tag {}", tag))),
        };

        if cursor != payload.len() {
            return Err(WalError::InvalidPayload(format!(
                "trailing bytes after entry: decoded {} bytes, payload is {} bytes",
                cursor,
                payload.len()
            )));
        }

        Ok(entry)
    }
}
