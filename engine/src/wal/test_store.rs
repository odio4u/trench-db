use std::collections::HashMap;
use std::sync::{Arc, Mutex};

use crate::traits::{Storage, Table};

/// A simple, event-free in-memory store used for WAL unit tests.
///
/// This intentionally does not use [`MemoryStore`] so that tests are not
/// affected by the storage event system's lifecycle.
#[derive(Debug, Default)]
pub struct TestStore {
    tables: Mutex<HashMap<String, Arc<TestTable>>>,
}

impl TestStore {
    pub fn new() -> Self {
        Self::default()
    }

    fn get_table(&self, table: &String) -> Option<Arc<TestTable>> {
        self.tables.lock().unwrap().get(table).cloned()
    }

    fn get_or_create_table(&self, table: &String) -> Arc<TestTable> {
        let mut tables = self.tables.lock().unwrap();
        tables
            .entry(table.clone())
            .or_insert_with(|| Arc::new(TestTable::new()))
            .clone()
    }
}

impl Table<String, Vec<u8>> for TestStore {
    fn is_empty(&self) -> bool {
        self.tables.lock().unwrap().is_empty()
    }

    fn len(&self) -> usize {
        self.tables.lock().unwrap().len()
    }

    fn get(&self, table: &String) -> Option<Arc<dyn Storage<String, Vec<u8>> + Send + Sync>> {
        self.get_table(table)
            .map(|t| t as Arc<dyn Storage<String, Vec<u8>> + Send + Sync>)
    }

    fn create(&self, table: &String) -> Arc<dyn Storage<String, Vec<u8>> + Send + Sync> {
        self.get_or_create_table(table) as Arc<dyn Storage<String, Vec<u8>> + Send + Sync>
    }

    fn remove(&self, table: &String) {
        self.tables.lock().unwrap().remove(table);
    }
}

#[derive(Debug, Default)]
struct TestTable {
    data: Mutex<HashMap<String, Arc<Vec<u8>>>>,
}

impl TestTable {
    fn new() -> Self {
        Self::default()
    }
}

impl Storage<String, Vec<u8>> for TestTable {
    fn get(&self, key: &String) -> Option<Arc<Vec<u8>>> {
        self.data.lock().unwrap().get(key).cloned()
    }

    fn insert(&self, key: String, value: Vec<u8>) {
        self.data.lock().unwrap().insert(key, Arc::new(value));
    }

    fn remove(&self, key: &String) -> Option<Arc<Vec<u8>>> {
        self.data.lock().unwrap().remove(key)
    }

    fn update(&self, key: String, value: Vec<u8>) {
        self.data.lock().unwrap().insert(key, Arc::new(value));
    }

    fn contains(&self, key: &String) -> bool {
        self.data.lock().unwrap().contains_key(key)
    }
}
