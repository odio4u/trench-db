//! A `DashMap`-backed, concurrent, in-memory implementation of `Storage`.

use std::hash::Hash;
use std::sync::Arc;

use dashmap::DashMap;

use crate::metrics::Metrics;
use crate::rec::collections;
use crate::traits::Table;

/// A generic, lock-striped, thread-safe in-memory store.
pub struct MemoryStore<K, V>
where
    K: Eq + Hash + Send + Sync,
    V: Send + Sync,
{
    map: DashMap<K, Arc<collections::Collection<K, V>>>,
    metrics: Arc<Metrics>,
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
            metrics: Arc::new(Metrics::new()),
        }
    }

    /// Returns the metrics instance shared by this store and its tables.
    pub fn metrics(&self) -> Arc<Metrics> {
        Arc::clone(&self.metrics)
    }

    /// Returns the number of tables currently stored.
    pub fn len(&self) -> usize {
        self.map.len()
    }

    /// Returns `true` if the store contains no tables.
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
    K: Eq + Hash + Clone + Send + Sync + crate::metrics::ByteSized + 'static,
    V: Send + Sync + crate::metrics::ByteSized + 'static,
{
    fn metrics(&self) -> Arc<Metrics> {
        self.metrics()
    }

    fn is_empty(&self) -> bool {
        self.is_empty()
    }

    fn len(&self) -> usize {
        self.len()
    }

    fn get(&self, table: &K) -> Option<Arc<dyn crate::traits::Storage<K, V> + Send + Sync>> {
        self.map
            .get(table)
            .map(|entry| Arc::clone(&*entry) as Arc<dyn crate::traits::Storage<K, V> + Send + Sync>)
    }

    fn create(&self, table: &K) -> Arc<dyn crate::traits::Storage<K, V> + Send + Sync> {
        let entry = self
            .map
            .entry(table.clone())
            .or_insert_with(|| Arc::new(collections::Collection::new(Arc::clone(&self.metrics))));
        Arc::clone(&*entry) as Arc<dyn crate::traits::Storage<K, V> + Send + Sync>
    }

    fn remove(&self, table: &K) {
        self.map.remove(table);
    }
}
