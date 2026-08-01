//! Panic isolation boundary for task execution.

use std::any::Any;
use std::panic::{catch_unwind, AssertUnwindSafe};
use crate::queue::event_queue::Task;

pub fn execute(task: Task) {
    let result = catch_unwind(AssertUnwindSafe(task));
    if let Err(payload) = result {
        report_panic(payload);
    }
}

fn report_panic(payload: Box<dyn Any + Send>) {
    let message = if let Some(message) = payload.downcast_ref::<&str>() {
        format!("task panicked: {message}")
    } else if let Some(message) = payload.downcast_ref::<String>() {
        format!("task panicked: {message}")
    } else {
        "task panicked with non-string payload".to_string()
    };

    eprintln!("[event loop] {message}");
}
