//! Equality-only secondary indexes for [`MemoryStore`](crate::memory::MemoryStore).
//!
//! Nothing is indexed by default. A value type opts in by implementing
//! [`Indexable`], and a caller explicitly registers the fields it wants
//! indexed via `MemoryStore::create_index`. Once registered, the store's
//! `insert`/`update`/`remove` path keeps every registered index in sync
//! automatically for the rest of that key's lifecycle — no manual
//! bookkeeping required at call sites.
//!
//! This is deliberately **equality-only**: an index answers "which keys
//! have `field == value`", not range/order queries. See
//! `doc/storage/storage.md` for why ordered (B-Tree-style) indexes are a
//! separate, not-yet-planned concern.

use std::hash::Hash;
use std::sync::Arc;

use dashmap::{DashMap, DashSet};

/// An equality-comparable value extracted from a record for indexing.
///
/// Kept as a small, fixed set of variants (rather than generic over an
/// arbitrary field type) so the index machinery only ever needs
/// `Eq`/`Hash`, matching the equality-only design.
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub enum IndexValue {
    Str(String),
    Int(i64),
    Bool(bool),
}

impl From<String> for IndexValue {
    fn from(value: String) -> Self {
        IndexValue::Str(value)
    }
}

impl From<&str> for IndexValue {
    fn from(value: &str) -> Self {
        IndexValue::Str(value.to_string())
    }
}

impl From<i64> for IndexValue {
    fn from(value: i64) -> Self {
        IndexValue::Int(value)
    }
}

impl From<i32> for IndexValue {
    fn from(value: i32) -> Self {
        IndexValue::Int(value as i64)
    }
}

impl From<bool> for IndexValue {
    fn from(value: bool) -> Self {
        IndexValue::Bool(value)
    }
}

/// Implemented by value types that expose named fields eligible for
/// secondary indexing.
///
/// Types with nothing worth indexing (e.g. opaque byte blobs) should
/// implement this trivially, always returning `None` — see the `Vec<u8>`
/// impl below.
pub trait Indexable {
    /// Returns the equality-index value stored at `field`, or `None` if
    /// this value has no such field (it is simply skipped for that index).
    fn index_value(&self, field: &str) -> Option<IndexValue>;
}

impl Indexable for Vec<u8> {
    fn index_value(&self, _field: &str) -> Option<IndexValue> {
        None
    }
}

impl Indexable for i32 {
    fn index_value(&self, _field: &str) -> Option<IndexValue> {
        None
    }
}

/// Owns the set of registered secondary indexes for a store keyed by `K`,
/// and keeps them in sync as records change.
///
/// Structured as `field name -> (index value -> matching primary keys)`.
/// No field is indexed until [`create_index`](Self::create_index) is
/// called for it.
pub struct IndexWorker<K>
where
    K: Eq + Hash + Clone,
{
    fields: DashMap<String, DashMap<IndexValue, DashSet<K>>>,
}

impl<K> IndexWorker<K>
where
    K: Eq + Hash + Clone,
{
    /// Creates a worker with no registered indexes.
    pub fn new() -> Self {
        Self {
            fields: DashMap::new(),
        }
    }

    /// Registers `field` for indexing. A no-op if already registered.
    ///
    /// Returns `true` if a new (initially empty) index was created, so the
    /// caller knows whether it still needs to backfill existing entries.
    pub fn create_index(&self, field: impl Into<String>) -> bool {
        let field = field.into();
        if self.fields.contains_key(&field) {
            return false;
        }
        self.fields.insert(field, DashMap::new());
        true
    }

    /// Drops `field`'s index entirely, discarding all entries.
    ///
    /// Returns `true` if an index existed and was removed.
    pub fn drop_index(&self, field: &str) -> bool {
        self.fields.remove(field).is_some()
    }

    /// Returns `true` if `field` currently has a registered index.
    pub fn has_index(&self, field: &str) -> bool {
        self.fields.contains_key(field)
    }

    /// Returns every primary key currently indexed under `field == value`.
    pub fn lookup(&self, field: &str, value: &IndexValue) -> Vec<K> {
        self.fields
            .get(field)
            .and_then(|buckets| {
                buckets
                    .get(value)
                    .map(|set| set.iter().map(|item| item.key().clone()).collect())
            })
            .unwrap_or_default()
    }

    /// Updates every registered index to reflect `key`'s value changing
    /// from `old` to `new`. Either side may be `None`: `insert` passes
    /// `old = None`, `remove` passes `new = None`, `update`/overwrite
    /// passes both.
    pub fn on_change<V: Indexable>(&self, key: &K, old: Option<&V>, new: Option<&V>) {
        for field_entry in self.fields.iter() {
            let field = field_entry.key();
            let buckets = field_entry.value();

            let old_value = old.and_then(|v| v.index_value(field));
            let new_value = new.and_then(|v| v.index_value(field));

            if old_value == new_value {
                continue;
            }

            if let Some(value) = old_value {
                if let Some(set) = buckets.get(&value) {
                    set.remove(key);
                    let now_empty = set.is_empty();
                    drop(set);
                    if now_empty {
                        buckets.remove(&value);
                    }
                }
            }

            if let Some(value) = new_value {
                buckets.entry(value).or_insert_with(DashSet::new).insert(key.clone());
            }
        }
    }

    /// Rebuilds `field`'s index from scratch using `entries`. Used when a
    /// new index is registered after data already exists in the store.
    /// A no-op if `field` isn't registered.
    pub fn backfill<V: Indexable>(&self, field: &str, entries: impl IntoIterator<Item = (K, Arc<V>)>) {
        let Some(buckets) = self.fields.get(field) else {
            return;
        };
        buckets.clear();
        for (key, value) in entries {
            if let Some(index_value) = value.index_value(field) {
                buckets.entry(index_value).or_insert_with(DashSet::new).insert(key);
            }
        }
    }
}

impl<K> Default for IndexWorker<K>
where
    K: Eq + Hash + Clone,
{
    fn default() -> Self {
        Self::new()
    }
}
