//! A `DashMap`-backed, concurrent, in-memory implementation of `Storage`.

use std::hash::Hash;

use dashmap::DashMap;

use crate::rec::collections;
use crate::traits::Table;

/// A generic, lock-striped, thread-safe in-memory store.
pub struct MemoryStore<K, V>
where
    K: Eq + Hash,
{
    map: DashMap<K, collections::Collection<K, V>>,
}


impl<K, V> MemoryStore<K, V>
where
    K: Eq + Hash,
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


impl <K, V> Default for MemoryStore<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}


impl <K, V> Table<K, V> for MemoryStore<K, V>
where
    K: Eq + Hash + Clone,
{
    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn new(&self, table: &K) -> collections::Collection<K, V> {
        if self.map.get(table).is_none() {
            self.map.insert(table.clone(), collections::Collection::new());
        }
        self.map.get(table).unwrap().clone()
    }

    fn clear(&self, table: &K) {
        self.map.remove(table);
    }
}