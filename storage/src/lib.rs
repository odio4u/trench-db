//! In-memory storage engine for TrenchDB.
//!
//! Phase 1: a minimal, generic, thread-safe key-value store with no
//! networking. See `doc/storage/storage.md` for the full design and phased
//! plan.

pub mod config;
pub mod metadata;
pub mod memory;
pub mod rec;
pub mod traits;
pub mod events;

use std::sync::Arc;

pub use memory::MemoryStore;
pub use rec::record::Record;
pub use traits::{Storage, Table};

/// Shared table registry type handed to every network handler.
pub type SharedStore = Arc<dyn Table<String, Vec<u8>> + Send + Sync>;
    