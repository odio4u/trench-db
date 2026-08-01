//! Task dispatching for the storage event loop.

use crate::queue::{panic_boundary, event_queue::Task};

pub struct Dispatcher;

impl Dispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch(&self, task: Task) {
        panic_boundary::execute(task);
    }
}
