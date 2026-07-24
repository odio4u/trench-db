
use std::hash::Hash;
use std::sync::Arc;
use dashmap::DashMap;
use crate::record::Record;
use crate::traits::Storage;


#[derive(Debug)]
pub struct Collection<K, V>
where
    K: Eq + Hash,
{
    map: DashMap<K, Record<V>>,
}


impl <K, V> Collection<K, V>
where
    K: Eq + Hash,
{
    /// Creates a new, empty collection.
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

impl<K, V> Clone for Collection<K, V>
where
    K: Eq + Hash + Clone,
{
    fn clone(&self) -> Self {
        Self {
            map: self.map.clone(),
        }
    }
}


impl <K, V> Default for Collection<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self::new()
    }
}

impl <K, V> Storage<K, V> for Collection<K, V>
where
    K: Eq + Hash + Clone,

{
    fn get(&self, key: &K) -> Option<Arc<V>> {
        self.map.get(key).map(|entry| Arc::clone(&entry.value))
    }

    fn insert(&self, key: K, value: V) {
        self.map.insert(key, Record::new(value));
    }

    fn remove(&self, key: &K) -> Option<Arc<V>> {
        self.map.remove(key).map(|(_, entry)| Arc::clone(&entry.value))
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