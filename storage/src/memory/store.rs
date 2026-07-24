//! A `DashMap`-backed, concurrent, in-memory implementation of `Storage`.

use std::hash::Hash;
use std::sync::Arc;

use dashmap::DashMap;

use crate::index::{IndexValue, IndexWorker, Indexable};
use crate::record::Record;
use crate::traits::Storage;

/// A generic, lock-striped, thread-safe in-memory store.
pub struct MemoryStore<K, V>
where
    K: Eq + Hash + Clone,
{
    map: DashMap<K, Record<V>>,
    indexes: IndexWorker<K>,
}

impl<K, V> MemoryStore<K, V>
where
    K: Eq + Hash + Clone,
{
    /// Creates a new, empty store.
    pub fn new() -> Self {
        Self {
            map: DashMap::new(),
            indexes: IndexWorker::new(),
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
    K: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}

impl<K, V> MemoryStore<K, V>
where
    K: Eq + Hash + Clone,
    V: Indexable,
{
    /// Registers a secondary index on `field`, backfilling it from any
    /// entries already present in the store. A no-op if `field` is
    /// already indexed.
    ///
    /// Once registered, `insert`/`update`/`remove` keep this index in sync
    /// automatically for the rest of the store's lifecycle.
    pub fn create_index(&self, field: impl Into<String>) {
        let field = field.into();
        if !self.indexes.create_index(field.clone()) {
            return;
        }

        let entries: Vec<(K, Arc<V>)> = self
            .map
            .iter()
            .map(|entry| (entry.key().clone(), Arc::clone(&entry.value)))
            .collect();
        self.indexes.backfill(&field, entries);
    }

    /// Drops `field`'s index entirely, discarding all indexed entries.
    ///
    /// Returns `true` if an index existed and was removed.
    pub fn drop_index(&self, field: &str) -> bool {
        self.indexes.drop_index(field)
    }

    /// Returns `true` if `field` currently has a registered index.
    pub fn has_index(&self, field: &str) -> bool {
        self.indexes.has_index(field)
    }

    /// Returns every value currently indexed under `field == value`.
    pub fn find_by_index(&self, field: &str, value: impl Into<IndexValue>) -> Vec<Arc<V>> {
        let value = value.into();
        self.indexes
            .lookup(field, &value)
            .into_iter()
            .filter_map(|key| self.get(&key))
            .collect()
    }
}

impl<K, V> Storage<K, V> for MemoryStore<K, V>
where
    K: Eq + Hash + Clone,
    V: Indexable,
{
    fn get(&self, key: &K) -> Option<Arc<V>> {
        self.map.get(key).map(|entry| Arc::clone(&entry.value))
    }

    fn insert(&self, key: K, value: V) {
        let previous = self.map.get(&key).map(|entry| Arc::clone(&entry.value));
        self.indexes.on_change(&key, previous.as_deref(), Some(&value));
        self.map.insert(key, Record::new(value));
    }

    fn remove(&self, key: &K) {
        if let Some((_, record)) = self.map.remove(key) {
            self.indexes.on_change(key, Some(record.value.as_ref()), None);
        }
    }

    fn update(&self, key: K, value: V) {
        let previous = self
            .map
            .get(&key)
            .map(|entry| (entry.version, Arc::clone(&entry.value)));
        self.indexes
            .on_change(&key, previous.as_ref().map(|(_, v)| v.as_ref()), Some(&value));

        match previous {
            Some((version, _)) => {
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

    #[derive(Debug, Clone, PartialEq, Eq)]
    struct User {
        name: String,
        age: i64,
    }

    impl Indexable for User {
        fn index_value(&self, field: &str) -> Option<IndexValue> {
            match field {
                "name" => Some(IndexValue::Str(self.name.clone())),
                "age" => Some(IndexValue::Int(self.age)),
                _ => None,
            }
        }
    }

    #[test]
    fn find_by_index_returns_empty_when_field_not_registered() {
        let store: MemoryStore<String, User> = MemoryStore::new();
        store.insert(
            "u1".to_string(),
            User {
                name: "alice".to_string(),
                age: 30,
            },
        );

        assert!(store.find_by_index("name", "alice").is_empty());
        assert!(!store.has_index("name"));
    }

    #[test]
    fn create_index_backfills_existing_entries() {
        let store: MemoryStore<String, User> = MemoryStore::new();
        store.insert(
            "u1".to_string(),
            User {
                name: "alice".to_string(),
                age: 30,
            },
        );
        store.insert(
            "u2".to_string(),
            User {
                name: "bob".to_string(),
                age: 40,
            },
        );

        store.create_index("name");

        assert!(store.has_index("name"));
        let found = store.find_by_index("name", "alice");
        assert_eq!(found.len(), 1);
        assert_eq!(
            found[0].as_ref(),
            &User {
                name: "alice".to_string(),
                age: 30,
            }
        );
    }

    #[test]
    fn index_stays_in_sync_across_insert_update_remove() {
        let store: MemoryStore<String, User> = MemoryStore::new();
        store.create_index("age");

        store.insert(
            "u1".to_string(),
            User {
                name: "alice".to_string(),
                age: 30,
            },
        );
        assert_eq!(store.find_by_index("age", 30_i64).len(), 1);

        // Overwriting via insert with a different age must move the entry
        // to the new bucket and clear it from the old one.
        store.insert(
            "u1".to_string(),
            User {
                name: "alice".to_string(),
                age: 31,
            },
        );
        assert!(store.find_by_index("age", 30_i64).is_empty());
        assert_eq!(store.find_by_index("age", 31_i64).len(), 1);

        // update() must do the same.
        store.update(
            "u1".to_string(),
            User {
                name: "alice".to_string(),
                age: 32,
            },
        );
        assert!(store.find_by_index("age", 31_i64).is_empty());
        assert_eq!(store.find_by_index("age", 32_i64).len(), 1);

        store.remove(&"u1".to_string());
        assert!(store.find_by_index("age", 32_i64).is_empty());
    }

    #[test]
    fn drop_index_clears_lookups() {
        let store: MemoryStore<String, User> = MemoryStore::new();
        store.create_index("name");
        store.insert(
            "u1".to_string(),
            User {
                name: "alice".to_string(),
                age: 30,
            },
        );
        assert_eq!(store.find_by_index("name", "alice").len(), 1);

        assert!(store.drop_index("name"));
        assert!(!store.has_index("name"));
        assert!(store.find_by_index("name", "alice").is_empty());

        // Removed data after the index is gone must not resurrect it.
        assert!(!store.drop_index("name"));
    }
}
