use std::fs::{File, OpenOptions};
use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use crate::wal::checksum::crc32;
use crate::wal::entry::WalEntry;
use crate::wal::error::WalError;
use crate::wal::format::{validate_header, write_header, ENTRY_OVERHEAD, WAL_HEADER_SIZE};
use crate::wal::options::WalOptions;

/// Append-only writer for a WAL file.
pub struct WalWriter {
    writer: BufWriter<File>,
    options: WalOptions,
    path: PathBuf,
    position: u64,
}

impl WalWriter {
    /// Opens (or creates) a WAL file at `path` and positions the writer at the
    /// end of the file. Existing non-empty files must start with a valid WAL
    /// header.
    pub fn open<P: AsRef<Path>>(path: P, options: WalOptions) -> Result<Self, WalError> {
        let path = path.as_ref().to_path_buf();
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        } else {
            return Err(WalError::PathMissing);
        }

        let file_exists = path.exists();
        let file = OpenOptions::new()
            .create(true)
            .append(true)
            .write(true)
            .open(&path)?;
        let mut writer = BufWriter::new(file);

        let position = if file_exists {
            let metadata = std::fs::metadata(&path)?;
            let size = metadata.len();
            if size == 0 {
                write_header(&mut writer)?;
                WAL_HEADER_SIZE as u64
            } else {
                validate_header(&mut File::open(&path)?)?;
                size
            }
        } else {
            write_header(&mut writer)?;
            WAL_HEADER_SIZE as u64
        };

        Ok(Self {
            writer,
            options,
            path,
            position,
        })
    }

    /// Convenience constructor with default options.
    pub fn create<P: AsRef<Path>>(path: P) -> Result<Self, WalError> {
        Self::open(path, WalOptions::default())
    }

    /// Appends a single entry and ensures it is durable according to
    /// [`WalOptions::sync_on_flush`].
    pub fn append(&mut self, entry: &WalEntry) -> Result<(), WalError> {
        self.append_batch(std::slice::from_ref(entry))
    }

    /// Appends a batch of entries and flushes (and optionally syncs) once for
    /// the entire batch.
    pub fn append_batch(&mut self, entries: &[WalEntry]) -> Result<(), WalError> {
        for entry in entries {
            let payload = entry.to_bytes();
            let len = payload.len();
            if len > self.options.entry_size_limit {
                return Err(WalError::InvalidPayload(format!(
                    "payload length {} exceeds configured limit {}",
                    len, self.options.entry_size_limit
                )));
            }
            if len > u32::MAX as usize {
                return Err(WalError::InvalidPayload(format!(
                    "payload length {} exceeds maximum {}",
                    len,
                    u32::MAX
                )));
            }

            let checksum = crc32(&payload);
            self.writer.write_all(&(len as u32).to_be_bytes())?;
            self.writer.write_all(&checksum.to_be_bytes())?;
            self.writer.write_all(&payload)?;
            self.position += ENTRY_OVERHEAD as u64 + len as u64;
        }
        self.flush()?;
        Ok(())
    }

    /// Flushes the internal buffer and optionally syncs to disk.
    pub fn flush(&mut self) -> Result<(), WalError> {
        self.writer.flush().map_err(WalError::Io)?;
        if self.options.sync_on_flush {
            self.sync()?;
        }
        Ok(())
    }

    /// Syncs the underlying file to disk without flushing the buffer first.
    /// Callers should normally use [`flush`](Self::flush).
    pub fn sync(&mut self) -> Result<(), WalError> {
        self.writer.get_ref().sync_data().map_err(WalError::Io)
    }

    /// Logical byte position of the next write (including the WAL header).
    pub fn position(&self) -> u64 {
        self.position
    }

    /// Path of the WAL file.
    pub fn path(&self) -> &Path {
        &self.path
    }

    /// Flushes and closes the writer.
    pub fn close(mut self) -> Result<(), WalError> {
        self.flush()
    }
}
