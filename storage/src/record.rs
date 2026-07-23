//! The generic record wrapper stored behind every key.
//!
//! Phase 1 keeps this intentionally minimal: just the value (behind an
//! `Arc` so reads never clone the underlying data) and a version counter
//! used by `update` to detect changes. Richer metadata (`created_at`,
//! `updated_at`, `expires_at`, flags, ...) is added in Phase 3.

use std::sync::Arc;

/// A single stored value, along with its version.
#[derive(Debug)]
pub struct Record<V> {
    pub value: Arc<V>,
    pub version: u64,
}

impl<V> Record<V> {
    /// Creates a new record starting at version 1.
    pub fn new(value: V) -> Self {
        Self {
            value: Arc::new(value),
            version: 1,
        }
    }

    /// Creates the next version of a record, succeeding `previous_version`.
    pub fn next(value: V, previous_version: u64) -> Self {
        Self {
            value: Arc::new(value),
            version: previous_version + 1,
        }
    }
}

impl<V> Clone for Record<V> {
    fn clone(&self) -> Self {
        Self {
            value: Arc::clone(&self.value),
            version: self.version,
        }
    }
}
