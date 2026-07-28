//! The generic storage abstraction all engines implement.

use std::hash::Hash;
use std::sync::Arc;

use crate::metrics::Metrics;

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
    /// Returns the metrics instance shared by this store and its tables.
    fn metrics(&self) -> Arc<Metrics>;

    /// Returns `true` if the store holds no tables.
    fn is_empty(&self) -> bool;

    /// Returns the number of tables currently stored.
    fn len(&self) -> usize;

    /// Returns an existing named table, if present.
    fn get(&self, table: &K) -> Option<Arc<dyn Storage<K, V> + Send + Sync>>;

    /// Creates a new named table or returns an existing one.
    fn create(&self, table: &K) -> Arc<dyn Storage<K, V> + Send + Sync>;

    /// Removes the named table and all of its entries.
    ///
    /// If the table does not exist, this is a no-op.
    fn remove(&self, table: &K);
}
