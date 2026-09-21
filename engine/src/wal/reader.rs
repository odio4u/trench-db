use std::fmt;
use std::fs::File;
use std::io::{BufReader, Read};
use std::path::Path;

use crate::wal::checksum::crc32;
use crate::wal::entry::WalEntry;
use crate::wal::error::WalError;
use crate::wal::format::{
    validate_header, ENTRY_CKSUM_SIZE, ENTRY_LEN_SIZE, ENTRY_OVERHEAD, WAL_HEADER_SIZE,
};

/// Streaming reader for a WAL file.
pub struct WalReader {
    reader: BufReader<File>,
    position: u64,
    eof: bool,
    entry_size_limit: usize,
}

impl fmt::Debug for WalReader {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.debug_struct("WalReader")
            .field("position", &self.position)
            .field("eof", &self.eof)
            .field("entry_size_limit", &self.entry_size_limit)
            .finish_non_exhaustive()
    }
}

impl WalReader {
    /// Opens a WAL file for reading and validates its header.
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self, WalError> {
        Self::open_with_options(path, crate::wal::options::WalOptions::default().entry_size_limit)
    }

    /// Opens a WAL file with a custom maximum payload size.
    pub fn open_with_options<P: AsRef<Path>>(
        path: P,
        entry_size_limit: usize,
    ) -> Result<Self, WalError> {
        let file = File::open(path)?;
        let mut reader = BufReader::new(file);
        validate_header(&mut reader)?;
        Ok(Self {
            reader,
            position: WAL_HEADER_SIZE as u64,
            eof: false,
            entry_size_limit,
        })
    }

    /// Current byte position of the reader.
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Returns the next entry, `None` at clean EOF, or an error if the file is
    /// truncated or corrupt.
    pub fn next_entry(&mut self) -> Result<Option<WalEntry>, WalError> {
        if self.eof {
            return Ok(None);
        }

        let entry_start = self.position;

        // Length header.
        let mut len_buf = [0u8; ENTRY_LEN_SIZE];
        match read_all(&mut self.reader, &mut len_buf)? {
            ReadStatus::Eof => {
                self.eof = true;
                return Ok(None);
            }
            ReadStatus::Partial => return Err(WalError::UnexpectedEof),
            ReadStatus::Complete => {}
        }
        let payload_len = u32::from_be_bytes(len_buf) as usize;
        if payload_len > self.entry_size_limit {
            return Err(WalError::CorruptEntry {
                position: entry_start,
                reason: format!(
                    "payload length {} exceeds configured limit {}",
                    payload_len, self.entry_size_limit
                ),
            });
        }

        // Checksum.
        let mut cksum_buf = [0u8; ENTRY_CKSUM_SIZE];
        match read_all(&mut self.reader, &mut cksum_buf)? {
            ReadStatus::Eof | ReadStatus::Partial => return Err(WalError::UnexpectedEof),
            ReadStatus::Complete => {}
        }
        let stored_checksum = u32::from_be_bytes(cksum_buf);

        // Payload.
        let mut payload = vec![0u8; payload_len];
        match read_all(&mut self.reader, &mut payload)? {
            ReadStatus::Eof | ReadStatus::Partial => return Err(WalError::UnexpectedEof),
            ReadStatus::Complete => {}
        }

        let computed_checksum = crc32(&payload);
        if computed_checksum != stored_checksum {
            return Err(WalError::CorruptEntry {
                position: entry_start,
                reason: format!(
                    "checksum mismatch: expected {:08x}, found {:08x}",
                    stored_checksum, computed_checksum
                ),
            });
        }

        let entry = WalEntry::from_bytes(&payload)?;
        self.position += ENTRY_OVERHEAD as u64 + payload_len as u64;
        Ok(Some(entry))
    }
}

pub(crate) enum ReadStatus {
    Eof,
    Partial,
    Complete,
}

/// Fills `buf` from `reader`, distinguishing clean EOF from partial reads.
pub(crate) fn read_all<R: Read>(reader: &mut R, buf: &mut [u8]) -> Result<ReadStatus, WalError> {
    let mut read = 0;
    while read < buf.len() {
        match reader.read(&mut buf[read..]) {
            Ok(0) => {
                return if read == 0 {
                    Ok(ReadStatus::Eof)
                } else {
                    Ok(ReadStatus::Partial)
                };
            }
            Ok(n) => read += n,
            Err(err) => return Err(err.into()),
        }
    }
    Ok(ReadStatus::Complete)
}
