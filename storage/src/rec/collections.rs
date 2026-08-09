
use std::hash::Hash;
use std::sync::Arc;
use dashmap::DashMap;
use crate::events;
use crate::rec::record::Record;
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
        events::publish_storage_event(b"insert".to_vec());
    }

    fn remove(&self, key: &K) -> Option<Arc<V>> {
        let result = self.map.remove(key).map(|(_, entry)| Arc::clone(&entry.value));
        events::publish_storage_event(b"delete".to_vec());
        result
    }

    fn update(&self, key: K, value: V) {
        match self.map.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(mut occupied) => {
                let previous_version = occupied.get().version;
                occupied.insert(Record::next(value, previous_version));
            }
            dashmap::mapref::entry::Entry::Vacant(vacant) => {
                vacant.insert(Record::new(value));
            }
        }
        events::publish_storage_event(b"update".to_vec());
    }

    fn contains(&self, key: &K) -> bool {
        self.map.contains_key(key)
    }
}