//! Minimal storage-layer event loop kernel.
//!
//! The kernel owns only a simple queue, a dispatcher, a panic boundary, and
//! lifecycle management. It is intentionally small and easy to understand.

use std::cell::RefCell;
use std::rc::Rc;

use crate::queue::{dispatcher::Dispatcher, event::RuntimeEvent, event_queue::EventQueue, lifecycle::{Lifecycle, StopPolicy}};
use crate::queue::state::State;

#[derive(Debug)]
pub enum EventLoopError {
    InvalidStateForPost,
    InvalidStateForRun(State),
}

struct EventLoopInner {
    lifecycle: RefCell<Lifecycle>,
    queue: RefCell<EventQueue>,
    events: RefCell<Vec<RuntimeEvent>>,
    dispatcher: Dispatcher,
}

#[derive(Clone)]
pub struct EventLoopHandle {
    inner: Rc<EventLoopInner>,
}

impl std::panic::UnwindSafe for EventLoopHandle {}
impl std::panic::RefUnwindSafe for EventLoopHandle {}

pub struct EventLoop {
    inner: Rc<EventLoopInner>,
}

impl std::panic::UnwindSafe for EventLoop {}
impl std::panic::RefUnwindSafe for EventLoop {}

impl EventLoop {
    pub fn new() -> Self {
        let inner = EventLoopInner {
            lifecycle: RefCell::new(Lifecycle::new()),
            queue: RefCell::new(EventQueue::new()),
            events: RefCell::new(Vec::new()),
            dispatcher: Dispatcher::new(),
        };

        Self {
            inner: Rc::new(inner),
        }
    }

    pub fn handle(&self) -> EventLoopHandle {
        EventLoopHandle {
            inner: self.inner.clone(),
        }
    }

    pub fn post<F>(&self, task: F) -> Result<(), EventLoopError>
    where
        F: FnOnce() + std::panic::UnwindSafe + 'static,
    {
        self.handle().post(task)
    }

    pub fn run(&self) -> Result<(), EventLoopError> {
        let state = self.inner.lifecycle.borrow().state();
        if state == State::Created {
            drop(self.inner.lifecycle.borrow());
            self.inner.lifecycle.borrow_mut().start();
        } else if state == State::Stopped {
            return Err(EventLoopError::InvalidStateForRun(state));
        }

        while self.inner.lifecycle.borrow().should_continue() {
            let task = match self.inner.queue.borrow_mut().pop() {
                Some(task) => task,
                None => break,
            };

            let event = self.inner.dispatcher.dispatch(task);
            self.emit(event);
        }

        self.inner.lifecycle.borrow_mut().complete_shutdown();
        Ok(())
    }

    pub fn stop(&self) {
        self.emit(RuntimeEvent::ShutdownRequested);
        self.inner.lifecycle.borrow_mut().request_stop(StopPolicy::Graceful);
    }

    pub fn stop_immediate(&self) {
        self.emit(RuntimeEvent::ShutdownRequested);
        self.inner.lifecycle.borrow_mut().request_stop(StopPolicy::Immediate);
    }

    pub fn take_events(&self) -> Vec<RuntimeEvent> {
        std::mem::take(&mut *self.inner.events.borrow_mut())
    }

    fn emit(&self, event: RuntimeEvent) {
        self.inner.events.borrow_mut().push(event);
    }
}

impl EventLoopHandle {
    pub fn post<F>(&self, task: F) -> Result<(), EventLoopError>
    where
        F: FnOnce() + std::panic::UnwindSafe + 'static,
    {
        let lifecycle = self.inner.lifecycle.borrow();
        if lifecycle.allows_post() {
            drop(lifecycle);
            self.inner.queue.borrow_mut().push(Box::new(task));
            Ok(())
        } else {
            drop(lifecycle);
            self.inner.events.borrow_mut().push(RuntimeEvent::QueueOverflow);
            Err(EventLoopError::InvalidStateForPost)
        }
    }
}
