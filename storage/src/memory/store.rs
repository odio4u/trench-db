//! A `DashMap`-backed, concurrent, in-memory implementation of `Storage`.

use std::hash::Hash;
use std::sync::Arc;

use dashmap::DashMap;

use crate::record::Record;
use crate::traits::Storage;

/// A generic, lock-striped, thread-safe in-memory store.
pub struct MemoryStore<K, V>
where
    K: Eq + Hash,
{
    map: DashMap<K, Record<V>>,
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

impl<K, V> Default for MemoryStore<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> Storage<K, V> for MemoryStore<K, V>
where
    K: Eq + Hash + Clone,
{
    fn get(&self, key: &K) -> Option<Arc<V>> {
        self.map.get(key).map(|entry| Arc::clone(&entry.value))
    }

    fn insert(&self, key: K, value: V) {
        self.map.insert(key, Record::new(value));
    }

    fn remove(&self, key: &K) {
        self.map.remove(key);
    }

    fn update(&self, key: K, value: V) {
        let previous_version = self.map.get(&key).map(|entry| entry.version);
        match previous_version {
            Some(version) => {
                self.map.insert(key, Record::next(value, version));
            }
            None => {
                self.map.insert(key, Record::new(value));
            }
        }
    }

    fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn insert_then_get_returns_value() {
        let store: MemoryStore<String, i32> = MemoryStore::new();
        store.insert("a".to_string(), 1);

        assert_eq!(store.get(&"a".to_string()).as_deref(), Some(&1));
    }

    #[test]
    fn get_missing_key_returns_none() {
        let store: MemoryStore<String, i32> = MemoryStore::new();

        assert_eq!(store.get(&"missing".to_string()), None);
    }

    #[test]
    fn insert_replaces_existing_value() {
        let store: MemoryStore<String, i32> = MemoryStore::new();
        store.insert("a".to_string(), 1);
        store.insert("a".to_string(), 2);

        assert_eq!(store.get(&"a".to_string()).as_deref(), Some(&2));
        assert_eq!(store.len(), 1);
    }

    #[test]
    fn update_existing_key_bumps_version() {
        let store: MemoryStore<String, i32> = MemoryStore::new();
        store.insert("a".to_string(), 1);
        store.update("a".to_string(), 2);

        assert_eq!(store.get(&"a".to_string()).as_deref(), Some(&2));
        assert_eq!(store.map.get(&"a".to_string()).unwrap().version, 2);
    }

    #[test]
    fn update_missing_key_behaves_like_insert() {
        let store: MemoryStore<String, i32> = MemoryStore::new();
        store.update("a".to_string(), 1);

        assert_eq!(store.get(&"a".to_string()).as_deref(), Some(&1));
        assert_eq!(store.map.get(&"a".to_string()).unwrap().version, 1);
    }

    #[test]
    fn remove_deletes_entry() {
        let store: MemoryStore<String, i32> = MemoryStore::new();
        store.insert("a".to_string(), 1);
        store.remove(&"a".to_string());

        assert!(!store.contains(&"a".to_string()));
        assert!(store.is_empty());
    }

    #[test]
    fn remove_missing_key_is_a_no_op() {
        let store: MemoryStore<String, i32> = MemoryStore::new();
        store.remove(&"missing".to_string());

        assert!(store.is_empty());
    }

    #[test]
    fn contains_reflects_presence() {
        let store: MemoryStore<String, i32> = MemoryStore::new();
        assert!(!store.contains(&"a".to_string()));

        store.insert("a".to_string(), 1);
        assert!(store.contains(&"a".to_string()));
    }
}
