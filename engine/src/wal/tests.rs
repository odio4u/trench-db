use std::fs::OpenOptions;
use std::path::PathBuf;

use crate::traits::Table;
use crate::wal::entry::WalEntry;
use crate::wal::error::WalError;
use crate::wal::format::{ENTRY_OVERHEAD, WAL_HEADER_SIZE, WAL_MAGIC};
use crate::wal::options::{ReplayMode, WalOptions};
use crate::wal::reader::WalReader;
use crate::wal::replay::{replay, replay_with_mode};
use crate::wal::test_store::TestStore;
use crate::wal::writer::WalWriter;

fn temp_path() -> (PathBuf, tempfile::TempDir) {
    let dir = tempfile::tempdir().unwrap();
    (dir.path().join("test.wal"), dir)
}

#[test]
fn roundtrip_all_entry_variants() {
    let entries = vec![
        WalEntry::CreateTable {
            table: "users".into(),
        },
        WalEntry::RemoveTable {
            table: "old".into(),
        },
        WalEntry::Insert {
            table: "users".into(),
            key: "alice".into(),
            value: b"data".to_vec(),
        },
        WalEntry::Update {
            table: "users".into(),
            key: "alice".into(),
            value: b"new".to_vec(),
        },
        WalEntry::Delete {
            table: "users".into(),
            key: "alice".into(),
        },
    ];
    for entry in entries {
        let bytes = entry.to_bytes();
        let decoded = WalEntry::from_bytes(&bytes).unwrap();
        assert_eq!(entry, decoded);
    }
}

#[test]
fn write_header_and_replay_empty() {
    let (path, _dir) = temp_path();
    {
        let _writer = WalWriter::create(&path).unwrap();
    }
    let store = TestStore::new();
    let summary = replay(&store, &path).unwrap();
    assert_eq!(summary.entries_applied, 0);
    assert_eq!(summary.final_position, WAL_HEADER_SIZE as u64);
}

#[test]
fn append_and_replay_applies_entries() {
    let (path, _dir) = temp_path();
    {
        let mut writer = WalWriter::create(&path).unwrap();
        writer
            .append(&WalEntry::CreateTable {
                table: "users".into(),
            })
            .unwrap();
        writer
            .append(&WalEntry::Insert {
                table: "users".into(),
                key: "alice".into(),
                value: b"123".to_vec(),
            })
            .unwrap();
    }
    let store = TestStore::new();
    let summary = replay(&store, &path).unwrap();
    assert_eq!(summary.entries_applied, 2);
    let table = store.get(&"users".to_string()).unwrap();
    assert_eq!(
        table.get(&"alice".to_string()).unwrap().as_ref(),
        &b"123".to_vec()
    );
}

#[test]
fn append_batch_is_durable() {
    let (path, _dir) = temp_path();
    {
        let mut writer = WalWriter::create(&path).unwrap();
        let entries = vec![
            WalEntry::CreateTable {
                table: "t".into(),
            },
            WalEntry::Insert {
                table: "t".into(),
                key: "k".into(),
                value: b"v".to_vec(),
            },
        ];
        writer.append_batch(&entries).unwrap();
    }
    let store = TestStore::new();
    let summary = replay(&store, &path).unwrap();
    assert_eq!(summary.entries_applied, 2);
}

#[test]
fn replay_missing_file_returns_empty_summary() {
    let path = PathBuf::from("/nonexistent/trenchdb/replay.wal");
    let store = TestStore::new();
    let summary = replay(&store, &path).unwrap();
    assert_eq!(summary.entries_applied, 0);
}

#[test]
fn replay_truncated_header_lenient() {
    let (path, _dir) = temp_path();
    {
        let mut writer = WalWriter::create(&path).unwrap();
        writer
            .append(&WalEntry::CreateTable {
                table: "t".into(),
            })
            .unwrap();
    }
    // Append two extra bytes so the next length header is truncated.
    let file = OpenOptions::new().write(true).open(&path).unwrap();
    let meta = file.metadata().unwrap();
    file.set_len(meta.len() + 2).unwrap();
    drop(file);

    let store = TestStore::new();
    let summary = replay_with_mode(&store, &path, ReplayMode::Lenient).unwrap();
    assert_eq!(summary.entries_applied, 1);
    assert!(summary.truncated_at.is_some());
}

#[test]
fn replay_truncated_header_strict() {
    let (path, _dir) = temp_path();
    {
        let mut writer = WalWriter::create(&path).unwrap();
        writer
            .append(&WalEntry::CreateTable {
                table: "t".into(),
            })
            .unwrap();
    }
    let file = OpenOptions::new().write(true).open(&path).unwrap();
    let meta = file.metadata().unwrap();
    file.set_len(meta.len() + 2).unwrap();
    drop(file);

    let store = TestStore::new();
    let err = replay_with_mode(&store, &path, ReplayMode::Strict).unwrap_err();
    assert!(matches!(err, WalError::UnexpectedEof));
}

#[test]
fn replay_corrupt_checksum_strict() {
    let (path, _dir) = temp_path();
    {
        let mut writer = WalWriter::create(&path).unwrap();
        writer
            .append(&WalEntry::CreateTable {
                table: "t".into(),
            })
            .unwrap();
    }
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[WAL_HEADER_SIZE + ENTRY_OVERHEAD] ^= 0xFF;
    std::fs::write(&path, &bytes).unwrap();

    let store = TestStore::new();
    let err = replay_with_mode(&store, &path, ReplayMode::Strict).unwrap_err();
    assert!(matches!(err, WalError::CorruptEntry { .. }));
}

#[test]
fn replay_corrupt_checksum_lenient() {
    let (path, _dir) = temp_path();
    {
        let mut writer = WalWriter::create(&path).unwrap();
        writer
            .append(&WalEntry::CreateTable {
                table: "t".into(),
            })
            .unwrap();
    }
    let mut bytes = std::fs::read(&path).unwrap();
    bytes[WAL_HEADER_SIZE + ENTRY_OVERHEAD] ^= 0xFF;
    std::fs::write(&path, &bytes).unwrap();

    let store = TestStore::new();
    let summary = replay_with_mode(&store, &path, ReplayMode::Lenient).unwrap();
    assert_eq!(summary.entries_applied, 0);
    assert!(summary.corruption_detected_at.is_some());
}

#[test]
fn invalid_header_rejected() {
    let (path, _dir) = temp_path();
    std::fs::write(&path, b"NOTAWAL!").unwrap();
    let err = WalReader::open(&path).unwrap_err();
    assert!(matches!(err, WalError::InvalidHeader { .. }));
}

#[test]
fn unsupported_version_rejected() {
    let (path, _dir) = temp_path();
    let mut header = [0u8; WAL_HEADER_SIZE];
    header[0..4].copy_from_slice(WAL_MAGIC);
    header[4] = 255;
    std::fs::write(&path, &header).unwrap();
    let err = WalReader::open(&path).unwrap_err();
    assert!(matches!(err, WalError::UnsupportedVersion { .. }));
}

#[test]
fn oversized_payload_rejected() {
    let (path, _dir) = temp_path();
    let mut options = WalOptions::default();
    options.entry_size_limit = 4;
    let mut writer = WalWriter::open(&path, options).unwrap();
    let err = writer
        .append(&WalEntry::Insert {
            table: "t".into(),
            key: "k".into(),
            value: b"hello".to_vec(),
        })
        .unwrap_err();
    assert!(matches!(err, WalError::InvalidPayload(_)));
}
