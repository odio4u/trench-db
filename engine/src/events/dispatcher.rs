

use crate::walmanager::{append, flush, WalEntry};

pub struct Dispatcher;

impl Dispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch(&self, id: u64, payload: Vec<u8>) {
        println!("Dispatching task with ID: {}, Payload: {:?}", id, payload);

        // Parse "op:table:key" into structured WAL fields.
        let parts: Vec<&[u8]> = payload.split(|&b| b == b':').collect();
        let (operation, composite_key) = match parts.as_slice() {
            [op, table, key, ..] => {
                let op_code = match *op {
                    b"insert" => Some(1u8),
                    b"delete" => Some(2u8),
                    b"update" => Some(3u8),
                    _ => None,
                };
                let mut ck = Vec::with_capacity(table.len() + 1 + key.len());
                ck.extend_from_slice(table);
                ck.push(b':');
                ck.extend_from_slice(key);
                (op_code, Some(ck))
            }
            _ => (None, None),
        };

        let entry = WalEntry {
            payload: payload.clone(),
            key: composite_key,
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