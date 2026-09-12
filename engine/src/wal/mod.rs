//! Write-ahead log (WAL) for TrenchDB.
//!
//! Each WAL file begins with a typed header so the format can be evolved and
//! partial files are detectable. Every entry is protected by a CRC-32 checksum
//! and a length prefix so replay can distinguish a clean end-of-file from
//! truncation or bit-rot.
//!
//! File layout:
//!
//! ```text
//! +----------------+----------------+----------------+----------------+
//! | magic (4)      | version (1)    | flags (1)      | reserved (2)   |
//! +----------------+----------------+----------------+----------------+
//! | len (4)        | crc32 (4)      | payload (len)                   |
//! +----------------+----------------+---------------------------------+
//! | ...                                                             |
//! +-----------------------------------------------------------------+
//! ```

pub mod entry;
pub mod error;
pub mod options;
pub mod reader;
pub mod replay;
pub(crate) mod writer;

pub(crate) mod checksum;
pub(crate) mod codec;
pub(crate) mod format;

#[cfg(test)]
mod test_store;
#[cfg(test)]
mod tests;

pub use entry::WalEntry;
pub use error::WalError;
pub use options::{ReplayMode, ReplaySummary, WalOptions};
pub use reader::WalReader;
pub use replay::{replay, replay_with_mode};

// The walmanager now writes through the `ewal` module. The original
// `wal::WalWriter` remains available for tests and internal readers.
