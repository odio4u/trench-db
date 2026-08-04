
// use crate::queue::{event::RuntimeEvent, event_queue::Task, panic_boundary};
use super::RuntimeEvent;
use super::queue::Task;
use super::errors;

pub struct Dispatcher;


impl Dispatcher {
    pub fn new() -> Self {
        Self
    }

    pub fn dispatch(&self, task: Task) -> RuntimeEvent {
        errors::execute(task)
    }
}
