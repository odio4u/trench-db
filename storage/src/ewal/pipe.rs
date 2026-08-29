
use std::fs::File;
use std::io::{BufWriter, Write};
use std::fs::OpenOptions;
use std::path::Path;
use std::sync::{Arc, Mutex};
use crate::ewal::error::WalError;
use crate::ewal::{WAL_HEADER_SIZE, WalEntry};


pub struct WALWriter {
    pub writer: Arc<Mutex<BufWriter<File>>>,
    pub position: u64,
    pub pathbuf: String,
}

impl WALWriter {

    pub fn init(path: &str) -> Result<Self, WalError> {
        let path = Path::new(path);
        if path.parent().is_none() {
            return Err(WalError::PathMissing);
        }

        if !path.exists() {
            if let Some(parent) = path.parent() {
                std::fs::create_dir_all(parent)?;
            }
        }

        let file = OpenOptions::new().create(true).append(true).write(true).open(path)?;
        let writer = BufWriter::new(file);

        let position = WAL_HEADER_SIZE as u64;


        Ok(Self {
            writer: Arc::new(Mutex::new(writer)),
            position,
            pathbuf: path.to_string_lossy().into_owned(),
        })
    }

    pub fn get_position(&self) -> u64 {
        self.position
    }

    pub fn append(&mut self, data: &WalEntry) -> Result<(), WalError> {
        let len = data.payload.len();
        if len > 2000 {
            return Err(WalError::InvalidPayload("WAL entry exceeds maximum size of 2000 bytes".to_string()));
        }
        let mut writer = self.writer.lock().unwrap();
        let encrypted_data = self.encrypt_data(data);
        writer.write_all(encrypted_data.as_bytes())?;
        self.position += encrypted_data.len() as u64;
        Ok(())
    }

    pub fn append_batch(&mut self, entries: &[WalEntry]) -> Result<(), WalError> {
        for entry in entries {
            self.append(entry)?;
        }
        Ok(())
    }

    pub fn flush(&mut self) -> Result<(), WalError> {
        let mut writer = self.writer.lock().unwrap();
        writer.flush().map_err(WalError::Io)
    }

    pub fn sync(&mut self) -> Result<(), WalError> {
        let writer = self.writer.lock().unwrap();
        writer.get_ref().sync_data().map_err(WalError::Io)
    }

    pub fn path(&self) -> &Path {
        Path::new(&self.pathbuf)
    }

    fn encrypt_data(&self, data: &WalEntry) -> String {
        let mut formatted = String::new();

        // encrypt the payload unimplemented, so just display the raw payload for now
        formatted.push_str(&format!("WAL Entry:\n"));
        formatted.push_str(&format!("  Payload Length: {}\n", data.payload.len()));
        formatted.push_str(&format!("  Payload: {:?}\n", data.payload));
        formatted.push_str(&format!("  Checksum: {:?}\n", data.checksum));
        formatted.push_str(&format!("  Key: {:?}\n", data.key));
        formatted.push_str(&format!("  Operation: {:?}\n", data.operation));
        formatted
    }
}