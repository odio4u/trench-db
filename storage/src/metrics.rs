//! In-process metrics for the storage engine.
//!
//! Tracks atomic counters for reads, writes, deletes, hits, misses,
//! approximate memory usage, and average operation latency.

use std::sync::atomic::{AtomicI64, AtomicU64, Ordering};

/// A type that can report its approximate in-memory byte size.
///
/// Used by `Collection` to maintain an approximate memory-usage counter.
pub trait ByteSized {
    /// Returns the approximate number of bytes this value occupies.
    fn byte_size(&self) -> usize;
}

impl ByteSized for Vec<u8> {
    fn byte_size(&self) -> usize {
        self.len()
    }
}

impl ByteSized for String {
    fn byte_size(&self) -> usize {
        self.len()
    }
}

impl<T: ByteSized + ?Sized> ByteSized for std::sync::Arc<T> {
    fn byte_size(&self) -> usize {
        self.as_ref().byte_size()
    }
}

impl ByteSized for () {
    fn byte_size(&self) -> usize {
        0
    }
}

/// Counters maintained by the storage engine.
#[derive(Debug, Default)]
pub struct Metrics {
    reads: AtomicU64,
    writes: AtomicU64,
    deletes: AtomicU64,
    hits: AtomicU64,
    misses: AtomicU64,
    total_latency_ns: AtomicU64,
    operation_count: AtomicU64,
    memory_usage_bytes: AtomicI64,
}

/// A point-in-time copy of all storage counters.
#[derive(Debug, Default, Clone, Copy)]
pub struct MetricsSnapshot {
    pub reads: u64,
    pub writes: u64,
    pub deletes: u64,
    pub hits: u64,
    pub misses: u64,
    pub average_latency_ns: u64,
    pub memory_usage_bytes: u64,
}

impl Metrics {
    /// Creates a new, zero-initialized metrics instance.
    pub fn new() -> Self {
        Self::default()
    }

    /// Records a read operation, classified as a hit or miss.
    pub fn record_read(&self, hit: bool, latency_ns: u64) {
        self.reads.fetch_add(1, Ordering::Relaxed);
        if hit {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }
        self.record_latency(latency_ns);
    }

    /// Records a write operation and the associated change in memory usage.
    pub fn record_write(&self, memory_delta: i64, latency_ns: u64) {
        self.writes.fetch_add(1, Ordering::Relaxed);
        self.update_memory_usage(memory_delta);
        self.record_latency(latency_ns);
    }

    /// Records a delete operation and the associated change in memory usage.
    pub fn record_delete(&self, memory_delta: i64, latency_ns: u64) {
        self.deletes.fetch_add(1, Ordering::Relaxed);
        self.update_memory_usage(memory_delta);
        self.record_latency(latency_ns);
    }

    /// Adds a latency sample to the running average.
    pub fn record_latency(&self, latency_ns: u64) {
        self.total_latency_ns
            .fetch_add(latency_ns, Ordering::Relaxed);
        self.operation_count.fetch_add(1, Ordering::Relaxed);
    }

    /// Adjusts the approximate memory-usage counter by `delta` bytes.
    pub fn update_memory_usage(&self, delta: i64) {
        self.memory_usage_bytes.fetch_add(delta, Ordering::Relaxed);
    }

    /// Returns a snapshot of all current counters.
    pub fn snapshot(&self) -> MetricsSnapshot {
        let total_latency = self.total_latency_ns.load(Ordering::Relaxed);
        let operation_count = self.operation_count.load(Ordering::Relaxed);
        let average_latency_ns = if operation_count == 0 {
            0
        } else {
            total_latency / operation_count
        };

        MetricsSnapshot {
            reads: self.reads.load(Ordering::Relaxed),
            writes: self.writes.load(Ordering::Relaxed),
            deletes: self.deletes.load(Ordering::Relaxed),
            hits: self.hits.load(Ordering::Relaxed),
            misses: self.misses.load(Ordering::Relaxed),
            average_latency_ns,
            memory_usage_bytes: self.memory_usage_bytes.load(Ordering::Relaxed).max(0) as u64,
        }
    }
}
