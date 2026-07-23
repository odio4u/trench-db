//! In-memory storage engine for TrenchDB.
//!
//! Phase 1: a minimal, generic, thread-safe key-value store with no
//! networking. See `doc/storage/storage.md` for the full design and phased
//! plan.

pub mod memory;
pub mod record;
pub mod traits;

pub use memory::MemoryStore;
pub use record::Record;
pub use traits::Storage;
