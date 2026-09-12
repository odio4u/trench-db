//! Singleton WAL manager.
//!
//! Provides a process-wide [`WalManager`](crate::walmanager::writers::WalManager)
//! that can be initialized once and accessed from anywhere.

pub mod writers;

pub use writers::{
    append, append_batch, flush, init_wal_manager, path, position, sync, wal_manager, WalManager,
};

// WAL entries and errors come from the `ewal` module.
pub use crate::ewal::{WalEntry, WalError};
