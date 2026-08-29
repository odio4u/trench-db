use std::path::{Path, PathBuf};
use std::sync::{Mutex, OnceLock};
use crate::ewal::{WalEntry, WalError, WALWriter};

pub struct WalManager {
    writer: Option<WALWriter>,
}

impl WalManager {
    pub fn default() -> Self {
        Self { writer: None }
    }

    
    pub fn create_writer<P: AsRef<Path>>(&mut self, path: P) -> Result<(), WalError> {
        if self.writer.is_some() {
            return Err(WalError::InvalidPayload(
                "WAL writer already created".to_string(),
            ));
        }
        let path_str = path.as_ref()
            .to_str()
            .ok_or_else(|| WalError::InvalidPayload("invalid UTF-8 WAL path".to_string()))?;
        self.writer = Some(WALWriter::init(path_str)?);
        Ok(())
    }

    pub fn is_initialized(&self) -> bool {
        self.writer.is_some()
    }

    pub fn append(&mut self, entry: &WalEntry) -> Result<(), WalError> {
        self.with_writer(|w| w.append(entry))
    }

    pub fn append_batch(&mut self, entries: &[WalEntry]) -> Result<(), WalError> {
        self.with_writer(|w| w.append_batch(entries))
    }

    pub fn flush(&mut self) -> Result<(), WalError> {
        self.with_writer(|w| w.flush())
    }

    pub fn sync(&mut self) -> Result<(), WalError> {
        self.with_writer(|w| w.sync())
    }

    pub fn position(&self) -> Result<u64, WalError> {
        self.writer.as_ref().ok_or_else(|| {
            WalError::InvalidPayload("WAL writer has not been created".to_string())
        })
        .map(|w| w.get_position())
    }

    /// Path of the WAL file.
    pub fn path(&self) -> Result<&Path, WalError> {
        self.writer.as_ref().ok_or_else(|| {
            WalError::InvalidPayload("WAL writer has not been created".to_string())
        })
        .map(|w| w.path())
    }

    fn with_writer<F, R>(&mut self, f: F) -> Result<R, WalError>
    where
        F: FnOnce(&mut WALWriter) -> Result<R, WalError>,
    {
        let writer = self.writer.as_mut().ok_or_else(|| {
            WalError::InvalidPayload("WAL writer has not been created".to_string())
        })?;
        f(writer)
    }
}

impl Default for WalManager {
    fn default() -> Self {
        Self::default()
    }
}

static WAL_MANAGER: OnceLock<Mutex<WalManager>> = OnceLock::new();
pub fn init_wal_manager<P: AsRef<Path>>(path: P) -> Result<(), WalError> {
    let mut manager = WalManager::default();
    manager.create_writer(path)?;
    WAL_MANAGER
        .set(Mutex::new(manager))
        .map_err(|_| WalError::InvalidPayload("WAL manager already initialized".to_string()))?;
    Ok(())
}

pub fn wal_manager() -> &'static Mutex<WalManager> {
    WAL_MANAGER.get().expect("WAL manager not initialized; call init_wal_manager first")
}

/// Convenience: appends a single entry to the global WAL.
pub fn append(entry: &WalEntry) -> Result<(), WalError> {
    wal_manager().lock().map_err(|_| WalError::InvalidPayload("WAL manager lock poisoned".to_string()))?.append(entry)
}

/// Convenience: appends a batch of entries to the global WAL.
pub fn append_batch(entries: &[WalEntry]) -> Result<(), WalError> {
    wal_manager().lock().map_err(|_| WalError::InvalidPayload("WAL manager lock poisoned".to_string()))?
    .append_batch(entries)
}

/// Convenience: flushes the global WAL.
pub fn flush() -> Result<(), WalError> {
    wal_manager().lock().map_err(|_| WalError::InvalidPayload("WAL manager lock poisoned".to_string()))?
    .flush()
}

/// Convenience: syncs the global WAL without flushing first.
pub fn sync() -> Result<(), WalError> {
    wal_manager().lock().map_err(|_| WalError::InvalidPayload("WAL manager lock poisoned".to_string()))?
    .sync()
}

/// Convenience: returns the logical byte position of the global WAL writer.
pub fn position() -> Result<u64, WalError> {
    wal_manager().lock().map_err(|_| WalError::InvalidPayload("WAL manager lock poisoned".to_string()))?
    .position()
}

/// Convenience: returns the path of the global WAL file.
pub fn path() -> Result<PathBuf, WalError> {
    wal_manager().lock().map_err(|_| WalError::InvalidPayload("WAL manager lock poisoned".to_string()))?.path()
    .map(Path::to_path_buf)
}