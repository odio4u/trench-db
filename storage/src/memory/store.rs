//! A `DashMap`-backed, concurrent, in-memory implementation of `Storage`.

use std::hash::Hash;
use std::sync::Arc;

use dashmap::DashMap;

use crate::rec::collections;
use crate::traits::Table;

/// A generic, lock-striped, thread-safe in-memory store.
pub struct MemoryStore<K, V>
where
    K: Eq + Hash + Send + Sync,
    V: Send + Sync,
{
    map: DashMap<K, Arc<collections::Collection<K, V>>>,
}


impl<K, V> MemoryStore<K, V>
where
    K: Eq + Hash + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    /// Creates a new, empty store.
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
        }
    }

    /// Returns the number of entries currently stored.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the store holds no entries.
    pub fn is_empty(&self) -> bool {
        self.map.is_empty()
    }
}


impl<K, V> Default for MemoryStore<K, V>
where
    K: Eq + Hash + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    fn default() -> Self {
        Self::new()
    }
}


impl<K, V> Table<K, V> for MemoryStore<K, V>
where
    K: Eq + Hash + Clone + Send + Sync + 'static,
    V: Send + Sync + 'static,
{
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn new(&self, table: &K) -> Arc<dyn crate::traits::Storage<K, V> + Send + Sync> {
        let entry = self
            .map
            .entry(table.clone())
            .or_insert_with(|| Arc::new(collections::Collection::new()));
        Arc::clone(&*entry) as Arc<dyn crate::traits::Storage<K, V> + Send + Sync>
    }

    fn clear(&self, table: &K) {
        self.map.remove(table);
    }
}