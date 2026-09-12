

use crate::walmanager::{append, flush, WalEntry};

pub struct Dispatcher;

impl Dispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch(&self, id: u64, payload: Vec<u8>) {
        println!("Dispatching task with ID: {}, Payload: {:?}", id, payload);

        let operation = payload
            .split(|&b| b == b':')
            .next()
            .and_then(|op| match op {
                b"insert" => Some(1u8),
                b"delete" => Some(2u8),
                b"update" => Some(3u8),
                _ => None,
            });

        let entry = WalEntry {
            payload: payload.clone(),
            key: None,
            operation,
            checksum: None,
        };

        if let Err(err) = append(&entry) {
            eprintln!("[dispatcher] failed to append WAL entry for task {}: {}", id, err);
            return;
        }

        if let Err(err) = flush() {
            eprintln!("[dispatcher] failed to flush WAL for task {}: {}", id, err);
        }
    }
}