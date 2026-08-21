use std::fmt;
use std::fs::{File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Write};
use std::path::Path;

#[derive(Debug)]
pub enum WalError {
    Io(io::Error),
    InvalidPayload(String),
    UnexpectedEof,
}

impl fmt::Display for WalError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            WalError::Io(err) => write!(f, "I/O error: {}", err),
            WalError::InvalidPayload(msg) => write!(f, "Invalid WAL payload: {}", msg),
            WalError::UnexpectedEof => write!(f, "Unexpected end of WAL file"),
        }
    }
}

impl std::error::Error for WalError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            WalError::Io(err) => Some(err),
            _ => None,
        }
    }
}

impl From<io::Error> for WalError {
    fn from(value: io::Error) -> Self {
        if value.kind() == io::ErrorKind::UnexpectedEof {
            WalError::UnexpectedEof
        } else {
            WalError::Io(value)
        }
    }
}

#[derive(Debug, PartialEq, Eq)]
pub enum WalEntry {
    CreateTable { table: String },
    RemoveTable { table: String },
    Insert { table: String, key: String, value: Vec<u8> },
    Update { table: String, key: String, value: Vec<u8> },
    Delete { table: String, key: String },
}

impl WalEntry {
    pub fn to_bytes(&self) -> Result<Vec<u8>, WalError> {
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
        Ok(buf)
    }

    pub fn from_bytes(payload: &[u8]) -> Result<Self, WalError> {
        let mut cursor = 0;
        let tag = *payload.get(cursor).ok_or_else(|| WalError::InvalidPayload("missing tag".to_string()))?;
        cursor += 1;

        let (table, size) = decode_string(payload, cursor)?;
        cursor += size;

        match tag {
            0 => Ok(WalEntry::CreateTable { table }),
            1 => Ok(WalEntry::RemoveTable { table }),
            2 => {
                let (key, key_size) = decode_string(payload, cursor)?;
                cursor += key_size;
                let (value, _) = decode_bytes(payload, cursor)?;
                Ok(WalEntry::Insert { table, key, value })
            }
            3 => {
                let (key, key_size) = decode_string(payload, cursor)?;
                cursor += key_size;
                let (value, _) = decode_bytes(payload, cursor)?;
                Ok(WalEntry::Update { table, key, value })
            }
            4 => {
                let (key, _) = decode_string(payload, cursor)?;
                Ok(WalEntry::Delete { table, key })
            }
            _ => Err(WalError::InvalidPayload(format!("unknown tag {}", tag))),
        }
    }
}

fn encode_string(buf: &mut Vec<u8>, value: &str) {
    let bytes = value.as_bytes();
    buf.extend_from_slice(&(bytes.len() as u32).to_be_bytes());
    buf.extend_from_slice(bytes);
}

fn encode_bytes(buf: &mut Vec<u8>, value: &[u8]) {
    buf.extend_from_slice(&(value.len() as u32).to_be_bytes());
    buf.extend_from_slice(value);
}

fn decode_string(payload: &[u8], offset: usize) -> Result<(String, usize), WalError> {
    let len = read_u32(payload, offset)? as usize;
    let start = offset + 4;
    let end = start.checked_add(len).ok_or_else(|| WalError::InvalidPayload("string length overflow".to_string()))?;
    let bytes = payload.get(start..end).ok_or_else(|| WalError::InvalidPayload("string bytes missing".to_string()))?;
    let string = String::from_utf8(bytes.to_vec()).map_err(|err| WalError::InvalidPayload(format!("invalid utf8: {}", err)))?;
    Ok((string, 4 + len))
}

fn decode_bytes(payload: &[u8], offset: usize) -> Result<(Vec<u8>, usize), WalError> {
    let len = read_u32(payload, offset)? as usize;
    let start = offset + 4;
    let end = start.checked_add(len).ok_or_else(|| WalError::InvalidPayload("bytes length overflow".to_string()))?;
    let bytes = payload.get(start..end).ok_or_else(|| WalError::InvalidPayload("bytes missing".to_string()))?;
    Ok((bytes.to_vec(), 4 + len))
}

fn read_u32(payload: &[u8], offset: usize) -> Result<u32, WalError> {
    let end = offset + 4;
    let bytes = payload.get(offset..end).ok_or_else(|| WalError::InvalidPayload("length header missing".to_string()))?;
    Ok(u32::from_be_bytes(bytes.try_into().unwrap()))
}

pub struct WalWriter {
    writer: BufWriter<File>,
}

impl WalWriter {
    pub fn new<P: AsRef<Path>>(path: P) -> Result<Self, WalError> {
        let path = path.as_ref();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }

        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(path)?;

        Ok(WalWriter {
            writer: BufWriter::new(file),
        })
    }

    pub fn append(&mut self, entry_payload: &[u8]) -> Result<(), WalError> {
        let len = entry_payload.len() as u32;
        self.writer.write_all(&len.to_be_bytes())?;
        self.writer.write_all(entry_payload)?;
        self.writer.flush()?;
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), WalError> {
        self.writer.flush().map_err(WalError::Io)
    }
}

pub fn replay(store: &dyn crate::traits::Table<String, Vec<u8>>, path: impl AsRef<Path>) -> Result<(), WalError> {
    let file = File::open(path)?;
    let mut reader = BufReader::new(file);

    loop {
        let mut len_buf = [0u8; 4];
        match reader.read_exact(&mut len_buf) {
            Ok(()) => {}
            Err(err) if err.kind() == io::ErrorKind::UnexpectedEof => break,
            Err(err) => return Err(err.into()),
        }

        let size = u32::from_be_bytes(len_buf) as usize;
        let mut payload = vec![0u8; size];
        reader.read_exact(&mut payload)?;
        let entry = WalEntry::from_bytes(&payload)?;
        apply_entry(store, entry);
    }

    Ok(())
}

fn apply_entry(store: &dyn crate::traits::Table<String, Vec<u8>>, entry: WalEntry) {
    match entry {
        WalEntry::CreateTable { table } => {
            store.create(&table);
        }
        WalEntry::RemoveTable { table } => {
            store.remove(&table);
        }
        WalEntry::Insert { table, key, value } => {
            store.create(&table).insert(key, value);
        }
        WalEntry::Update { table, key, value } => {
            store.create(&table).update(key, value);
        }
        WalEntry::Delete { table, key } => {
            if let Some(table_store) = store.get(&table) {
                table_store.remove(&key);
            }
        }
    }
}
