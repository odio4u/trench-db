//! The generic storage abstraction all engines implement.

use std::sync::Arc;
use std::hash::Hash;

/// A generic, thread-safe key-value store.
///
/// Implementations must guarantee the read path (`get`/`contains`) never
/// panics or unwraps, since it sits on the hot path.
pub trait Storage<K, V>
where
    K: Eq + Hash,
{
    /// Returns the value for `key`, if present.
    fn get(&self, key: &K) -> Option<Arc<V>>;

    /// Inserts `value` under `key`, replacing any existing entry.
    fn insert(&self, key: K, value: V);

    /// Removes `key`, if present.
    fn remove(&self, key: &K) -> Option<Arc<V>>;

    /// Replaces the value stored at `key` with `value`.
    ///
    /// Behaves like `insert` when `key` did not previously exist.
    fn update(&self, key: K, value: V);

    /// Returns `true` if `key` is present.
    fn contains(&self, key: &K) -> bool;
}

pub trait Table<K, V>
where
    K: Eq + Hash,
{
    /// Returns `true` if the store holds no tables.
    fn is_empty(&self) -> bool;

    /// Returns the number of tables currently stored.
    fn len(&self) -> usize;

    /// Returns an existing named table, if present.
    fn get(&self, table: &K) -> Option<Arc<dyn Storage<K, V> + Send + Sync>>;

    /// Creates a new named table or returns an existing one.
    fn create(&self, table: &K) -> Arc<dyn Storage<K, V> + Send + Sync>;

    /// Drops all entries in the named table.
    fn clear(&self, table: &K);
}
