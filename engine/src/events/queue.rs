//! A bounded FIFO event queue for the loop kernel.

use std::{collections::VecDeque, sync::{Arc, Condvar, Mutex, atomic::{AtomicUsize, Ordering}}};

pub(crate) struct Queue<T> {
    pub(crate) queue: Mutex<VecDeque<T>>,
    pub(crate) cvar: Condvar,
    pub(crate) queue_stats: Mutex<QueueStats>,
    /// Number of active runners attached to this queue. Used by the supervisor
    /// to decide whether a new runner needs to be spawned.
    pub(crate) active_runners: AtomicUsize,
}

pub struct QueueStats {
    pub total_pushed: u64,
    pub total_popped: u64,
    pub max_queue_size: usize,
}

pub struct SharedQueue<T> {
    queue: Arc<Queue<T>>,
    max_capacity: usize,
}

impl<T> SharedQueue<T> {

    pub fn new() -> Self {
        Self::with_capacity(usize::MAX)
    }

    /// Creates a bounded queue with the given maximum capacity.
    /// Capacity of 0 is treated as unbounded (same as `new`).
    pub fn with_capacity(max_capacity: usize) -> Self {
        let queue = Arc::new(Queue {
            queue: Mutex::new(VecDeque::new()),
            cvar: Condvar::new(),
            queue_stats: Mutex::new(QueueStats {
                total_pushed: 0,
                total_popped: 0,
                max_queue_size: 0,
            }),
            active_runners: AtomicUsize::new(0),
        });
        Self { queue, max_capacity }
    }

    pub(crate) fn inner(&self) -> &Arc<Queue<T>> {
        &self.queue
    }

    pub fn capacity(&self) -> usize {
        self.max_capacity
    }

    pub fn active_runners(&self) -> usize {
        self.queue.active_runners.load(Ordering::Relaxed)
    }
}


