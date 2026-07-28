use std::hash::Hash;
use std::sync::Arc;
use std::time::Instant;

use dashmap::DashMap;

use crate::metrics::{ByteSized, Metrics};
use crate::rec::record::Record;
use crate::traits::Storage;

/// Approximate per-entry overhead in bytes for a `DashMap` node plus a `Record`.
const ENTRY_OVERHEAD_BYTES: usize = 80;

#[derive(Debug)]
pub struct Collection<K, V>
where
    K: Eq + Hash,
{
    map: DashMap<K, Record<V>>,
    metrics: Arc<Metrics>,
}

impl<K, V> Collection<K, V>
where
    K: Eq + Hash,
{
    /// Creates a new, empty collection.
    pub fn new(metrics: Arc<Metrics>) -> Self {
        Self {
            map: DashMap::new(),
            metrics,
        }
    }

    /// Returns the metrics instance shared by this collection.
    pub fn metrics(&self) -> Arc<Metrics> {
        Arc::clone(&self.metrics)
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
            metrics: Arc::clone(&self.metrics),
        }
    }
}

impl<K, V> Default for Collection<K, V>
where
    K: Eq + Hash,
{
    fn default() -> Self {
        Self::new(Arc::new(Metrics::new()))
    }
}

impl<K, V> Storage<K, V> for Collection<K, V>
where
    K: Eq + Hash + Clone + ByteSized,
    V: Send + Sync + ByteSized,
{
    fn get(&self, key: &K) -> Option<Arc<V>> {
        let start = Instant::now();
        let result = self.map.get(key).map(|entry| Arc::clone(&entry.value));
        self.metrics
            .record_read(result.is_some(), elapsed_ns(start));
        result
    }

    fn insert(&self, key: K, value: V) {
        let start = Instant::now();
        let key_size = key.byte_size();
        let new_entry_size = key_size + value.byte_size() + ENTRY_OVERHEAD_BYTES;

        let delta = match self.map.insert(key, Record::new(value)) {
            Some(old) => {
                let old_entry_size = key_size + old.value.byte_size() + ENTRY_OVERHEAD_BYTES;
                new_entry_size as i64 - old_entry_size as i64
            }
            None => new_entry_size as i64,
        };

        self.metrics.record_write(delta, elapsed_ns(start));
    }

    fn remove(&self, key: &K) -> Option<Arc<V>> {
        let start = Instant::now();
        let result = self
            .map
            .remove(key)
            .map(|(_, entry)| Arc::clone(&entry.value));
        let delta = result
            .as_ref()
            .map(|value| {
                -(key.byte_size() as i64 + value.byte_size() as i64 + ENTRY_OVERHEAD_BYTES as i64)
            })
            .unwrap_or(0);
        self.metrics.record_delete(delta, elapsed_ns(start));
        result
    }

    fn update(&self, key: K, value: V) {
        let start = Instant::now();
        match self.map.entry(key) {
            dashmap::mapref::entry::Entry::Occupied(mut occupied) => {
                let old = occupied.get();
                let old_size = old.value.byte_size();
                let new_size = value.byte_size();
                let previous_version = old.version;
                occupied.insert(Record::next(value, previous_version));
                self.metrics
                    .record_write(new_size as i64 - old_size as i64, elapsed_ns(start));
            }
            dashmap::mapref::entry::Entry::Vacant(vacant) => {
                let key_size = vacant.key().byte_size();
                let delta = key_size + value.byte_size() + ENTRY_OVERHEAD_BYTES;
                vacant.insert(Record::new(value));
                self.metrics.record_write(delta as i64, elapsed_ns(start));
            }
        }
    }

    fn contains(&self, key: &K) -> bool {
        let start = Instant::now();
        let result = self.map.contains_key(key);
        self.metrics.record_read(result, elapsed_ns(start));
        result
    }
}

fn elapsed_ns(start: Instant) -> u64 {
    start.elapsed().as_nanos() as u64
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn metrics_count_reads_and_writes() {
        let collection = Collection::<String, Vec<u8>>::new(Arc::new(Metrics::new()));
        collection.get(&"missing".to_string());
        collection.insert("key".to_string(), b"value".to_vec());
        collection.get(&"key".to_string());

        let snapshot = collection.metrics().snapshot();
        assert_eq!(snapshot.reads, 2);
        assert_eq!(snapshot.misses, 1);
        assert_eq!(snapshot.hits, 1);
        assert_eq!(snapshot.writes, 1);
    }

    #[test]
    fn metrics_count_deletes() {
        let collection = Collection::<String, Vec<u8>>::new(Arc::new(Metrics::new()));
        collection.insert("key".to_string(), b"value".to_vec());
        collection.remove(&"key".to_string());

        let snapshot = collection.metrics().snapshot();
        assert_eq!(snapshot.deletes, 1);
    }

    #[test]
    fn metrics_track_memory_usage() {
        let collection = Collection::<String, Vec<u8>>::new(Arc::new(Metrics::new()));
        collection.insert("key".to_string(), b"value".to_vec());
        let after_insert = collection.metrics().snapshot().memory_usage_bytes;
        assert!(after_insert > 0);

        collection.remove(&"key".to_string());
        let after_remove = collection.metrics().snapshot().memory_usage_bytes;
        assert_eq!(after_remove, 0);
    }

    #[test]
    fn reinsert_does_not_inflate_memory_usage() {
        let collection = Collection::<String, Vec<u8>>::new(Arc::new(Metrics::new()));
        collection.insert("key".to_string(), b"value".to_vec());
        let after_first = collection.metrics().snapshot().memory_usage_bytes;

        collection.insert("key".to_string(), b"value".to_vec());
        let after_second = collection.metrics().snapshot().memory_usage_bytes;
        assert_eq!(after_first, after_second);

        collection.remove(&"key".to_string());
        let after_remove = collection.metrics().snapshot().memory_usage_bytes;
        assert_eq!(after_remove, 0);
    }

    #[test]
    fn reinsert_with_larger_value_updates_memory_usage() {
        let collection = Collection::<String, Vec<u8>>::new(Arc::new(Metrics::new()));
        collection.insert("key".to_string(), b"value".to_vec());
        let after_first = collection.metrics().snapshot().memory_usage_bytes;

        collection.insert("key".to_string(), b"a much longer value".to_vec());
        let after_second = collection.metrics().snapshot().memory_usage_bytes;
        assert!(after_second > after_first);

        collection.remove(&"key".to_string());
        let after_remove = collection.metrics().snapshot().memory_usage_bytes;
        assert_eq!(after_remove, 0);
    }

    #[test]
    fn metrics_track_average_latency() {
        let collection = Collection::<String, Vec<u8>>::new(Arc::new(Metrics::new()));
        collection.insert("key".to_string(), b"value".to_vec());
        collection.get(&"key".to_string());
        collection.contains(&"key".to_string());

        let snapshot = collection.metrics().snapshot();
        assert_eq!(snapshot.reads, 2);
        assert_eq!(snapshot.writes, 1);
        assert!(snapshot.average_latency_ns < 1_000_000_000);
    }
}
