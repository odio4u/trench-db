//! A simple FIFO event queue for the loop kernel.

use std::collections::VecDeque;

pub type Task = Box<dyn FnOnce() + std::panic::UnwindSafe + 'static>;

pub struct EventQueue {
    queue: VecDeque<Task>,
}

impl EventQueue {
    pub fn new() -> Self {
        Self {
            queue: VecDeque::new(),
        }
    }

    pub fn push(&mut self, task: Task) {
        self.queue.push_back(task);
    }

    pub fn pop(&mut self) -> Option<Task> {
        self.queue.pop_front()
    }

    pub fn is_empty(&self) -> bool {
        self.queue.is_empty()
    }
}
