//! Task dispatching for the storage event loop.

use crate::queue::{event::{RuntimeEvent}, event_queue::Task, panic_boundary};

pub struct Dispatcher;

impl Dispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch(&self, task: Task) -> RuntimeEvent {
        panic_boundary::execute(task)
    }
}
