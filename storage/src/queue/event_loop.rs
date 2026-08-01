//! Minimal storage-layer event loop kernel.
//!
//! The kernel owns only a simple queue, a dispatcher, a panic boundary, and
//! lifecycle management. It is intentionally small and easy to understand.

use crate::queue::{dispatcher::Dispatcher, event_queue::EventQueue, lifecycle::Lifecycle};
use crate::queue::state::State;

#[derive(Debug)]
pub enum EventLoopError {
    InvalidStateForPost,
    InvalidStateForRun(State),
}

pub struct EventLoop {
    lifecycle: Lifecycle,
    queue: EventQueue,
    dispatcher: Dispatcher,
}

impl EventLoop {
    pub fn new() -> Self {
        Self {
            lifecycle: Lifecycle::new(),
            queue: EventQueue::new(),
            dispatcher: Dispatcher::new(),
        }
    }

    pub fn post<F>(&mut self, task: F) -> Result<(), EventLoopError>
    where
        F: FnOnce() + std::panic::UnwindSafe + 'static,
    {
        if self.lifecycle.allows_post() {
            self.queue.push(Box::new(task));
            Ok(())
        } else {
            Err(EventLoopError::InvalidStateForPost)
        }
    }

    pub fn run(&mut self) -> Result<(), EventLoopError> {
        if !self.lifecycle.is_running() {
            self.lifecycle.start();
        }

        if !matches!(self.lifecycle.state, State::Running) {
            return Err(EventLoopError::InvalidStateForRun(self.lifecycle.state));
        }

        while let Some(task) = self.queue.pop() {
            self.dispatcher.dispatch(task);

            if !self.lifecycle.is_running() && self.queue.is_empty() {
                break;
            }
        }

        self.lifecycle.request_stop();
        Ok(())
    }

    pub fn stop(&mut self) {
        self.lifecycle.request_stop();
    }
}
