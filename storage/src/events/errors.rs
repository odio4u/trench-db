use std::panic::{catch_unwind, AssertUnwindSafe};
use std::any::Any;
use super::queue::Task;
use super::RuntimeEvent;

fn report_panic(payload: Box<dyn Any + Send>) {
    let message = if let Some(message) = payload.downcast_ref::<&str>() {
        format!("task panicked: {message}")
    } else if let Some(message) = payload.downcast_ref::<String>() {
        format!("task panicked: {message}")
    } else {
        format!("task panicked with unknown payload: {:?}", payload)
    };

    eprintln!("[event loop] {message}");
}


pub fn execute(task: Task) -> RuntimeEvent {
    let result = catch_unwind(AssertUnwindSafe(task));
    if let Err(payload) = result {
        report_panic(payload);
        RuntimeEvent::TaskFailed
    } else {
        RuntimeEvent::TaskCompleted
    }
}