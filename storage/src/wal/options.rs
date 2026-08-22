/// How to handle corrupt or truncated entries during replay.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ReplayMode {
    /// Any corruption or truncation is reported as an error.
    Strict,
    /// Stop replay at the first corrupt or truncated entry and return success.
    /// The caller is responsible for truncating or repairing the WAL.
    Lenient,
}

/// Configuration options for the WAL writer and reader.
#[derive(Debug, Clone)]
pub struct WalOptions {
    /// Sync the file to disk after every batch/flush. Enabled by default.
    pub sync_on_flush: bool,
    /// How to handle corrupt or truncated entries during replay.
    pub replay_mode: ReplayMode,
    /// Largest payload that may be written or read, in bytes. Acts as a guard
    /// against corrupted length headers. Default: 256 MiB.
    pub entry_size_limit: usize,
}

impl Default for WalOptions {
    fn default() -> Self {
        Self {
            sync_on_flush: true,
            replay_mode: ReplayMode::Lenient,
            entry_size_limit: 256 * 1024 * 1024,
        }
    }
}

/// Summary returned after a successful WAL replay.
#[derive(Debug, Clone, Default)]
pub struct ReplaySummary {
    /// Number of entries successfully applied to the store.
    pub entries_applied: u64,
    /// Byte position of the reader after replay finishes.
    pub final_position: u64,
    /// Offset where a truncated entry began, if any.
    pub truncated_at: Option<u64>,
    /// Offset where a checksum mismatch began, if any.
    pub corruption_detected_at: Option<u64>,
}
