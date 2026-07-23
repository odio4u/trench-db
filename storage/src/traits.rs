//! The generic storage abstraction all engines implement.

use std::sync::Arc;

/// A generic, thread-safe key-value store.
///
/// Implementations must guarantee the read path (`get`/`contains`) never
/// panics or unwraps, since it sits on the hot path.
pub trait Storage<K, V> {
    /// Returns the value for `key`, if present.
    fn get(&self, key: &K) -> Option<Arc<V>>;

    /// Inserts `value` under `key`, replacing any existing entry.
    fn insert(&self, key: K, value: V);

    /// Removes `key`, if present.
    fn remove(&self, key: &K);

    /// Replaces the value stored at `key` with `value`.
    ///
    /// Behaves like `insert` when `key` did not previously exist.
    fn update(&self, key: K, value: V);

    /// Returns `true` if `key` is present.
    fn contains(&self, key: &K) -> bool;
}
