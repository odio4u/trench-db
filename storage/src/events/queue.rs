//! A simple FIFO event queue for the loop kernel.

use std::{collections::VecDeque, sync::{Arc, Condvar, Mutex}};

pub struct Queue<T> {
    pub queue: Mutex<VecDeque<T>>,
    pub cvar: Condvar,
    pub queue_stats: Mutex<QueueStats>,
}


pub struct QueueStats {
    pub total_pushed: u64,
    pub total_popped: u64,
    pub max_queue_size: usize,
}

pub struct SharedQueue<T> {
    pub queue: Arc<Queue<T>>,
}

pub struct ConsumerHandle<T> {
   pub data: Arc<Queue<T>>,
}


impl<T> SharedQueue<T> {

    pub fn new() -> Self {
        let queue = Arc::new(Queue {
            queue: Mutex::new(VecDeque::new()),
            cvar: Condvar::new(),
            queue_stats: Mutex::new(QueueStats {
                total_pushed: 0,
                total_popped: 0,
                max_queue_size: 0,
            }),
        });
        Self { queue }
    }
}


