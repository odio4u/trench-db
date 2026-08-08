use std::sync::{Arc, Mutex};
use std::sync::atomic::Ordering;
use std::thread::{self, JoinHandle};
use std::time::Duration;

use super::consumer::{ConsumerHandle, RecvTimeoutError};
use super::dispatcher::Dispatcher;
use super::lifecycle::{Lifecycle, StopPolicy};
use super::producer::{ProducerHandle, PushError};
use super::queue::SharedQueue;
use super::Task;

const RECV_TIMEOUT: Duration = Duration::from_millis(100);

/// A single-node event loop that owns the consumer side of a `SharedQueue`
/// and dispatches tasks to the configured dispatcher.
pub struct EventLoop {
    lifecycle: Lifecycle,
    producer: ProducerHandle<Task>,
    consumer: ConsumerHandle<Task>,
    dispatcher: Dispatcher,
}

impl EventLoop {
    pub fn new(queue: &SharedQueue<Task>, lifecycle: Lifecycle) -> Self {
        Self {
            lifecycle,
            producer: queue.producer_handle(),
            consumer: queue.consumer_handle(),
            dispatcher: Dispatcher::new(),
        }
    }

    pub fn start(&mut self) {
        self.lifecycle.start();
    }

    pub fn request_stop(&mut self, policy: StopPolicy) {
        self.lifecycle.request_stop(policy);
    }

    pub fn producer(&self) -> ProducerHandle<Task> {
        ProducerHandle {
            data: self.producer.data.clone(),
            capacity: self.producer.capacity,
        }
    }

    /// Posts a task only while the lifecycle allows posting.
    pub fn post_task(&mut self, task: Task) -> Result<(), PushError<Task>> {
        if !self.lifecycle.allows_post() {
            return Err(PushError::Full(task));
        }
        self.producer.push(task)
    }

    pub fn run(&mut self) {
        self.producer
            .data
            .active_runners
            .fetch_add(1, Ordering::SeqCst);

        while self.lifecycle.should_continue() {
            match self.consumer.recv_timeout(RECV_TIMEOUT) {
                Ok(task) => {
                    self.dispatch(task);
                    // In graceful stop mode, if the queue is now empty we can exit.
                    if self.lifecycle.state() == super::lifecycle::State::Stopping && self.producer.is_empty()
                    {
                        break;
                    }
                }
                Err(RecvTimeoutError::Timeout) => {}
            }
        }

        self.lifecycle.complete_shutdown();
        self.producer
            .data
            .active_runners
            .fetch_sub(1, Ordering::SeqCst);
    }

    fn dispatch(&self, task: Task) {
        self.dispatcher.dispatch(task.id, task.payload);
    }
}

/// A supervisor that owns an `EventLoop` running on a dedicated thread.
/// If the runner thread panics or exits unexpectedly, the next `push` will
/// spawn a replacement automatically.
pub struct EventLoopSupervisor {
    shared_queue: Arc<SharedQueue<Task>>,
    lifecycle: Lifecycle,
    handle: Mutex<Option<JoinHandle<()>>>,
}

impl EventLoopSupervisor {
    pub fn new(queue: SharedQueue<Task>) -> Self {
        Self {
            shared_queue: Arc::new(queue),
            lifecycle: Lifecycle::new(),
            handle: Mutex::new(None),
        }
    }

    pub fn start(&self) {
        self.ensure_runner_alive();
    }

    /// Pushes a task into the queue. If the runner thread is no longer alive,
    /// a new one is spawned before returning.
    pub fn push(&self, task: Task) -> Result<(), PushError<Task>> {
        let mut handle_guard = self.handle.lock().unwrap();
        if Self::runner_is_dead(handle_guard.as_ref()) {
            *handle_guard = Some(self.spawn_runner());
        }

        let result = self.shared_queue.producer_handle().push(task);

        if Self::runner_is_dead(handle_guard.as_ref()) {
            *handle_guard = Some(self.spawn_runner());
        }

        result
    }

    pub fn request_stop(&self) {
        self.lifecycle.request_stop(StopPolicy::Graceful);
        if let Some(handle) = self.handle.lock().unwrap().take() {
            let _ = handle.join();
        }
    }

    fn ensure_runner_alive(&self) {
        let mut handle_guard = self.handle.lock().unwrap();
        if Self::runner_is_dead(handle_guard.as_ref()) {
            *handle_guard = Some(self.spawn_runner());
        }
    }

    fn runner_is_dead(handle: Option<&JoinHandle<()>>) -> bool {
        if let Some(handle) = handle {
            !handle.is_alive()
        } else {
            true
        }
    }

    fn spawn_runner(&self) -> JoinHandle<()> {
        let queue = Arc::clone(&self.shared_queue);
        let lifecycle = self.lifecycle.clone();

        thread::spawn(move || {
            let mut event_loop = EventLoop::new(&queue, lifecycle);
            event_loop.start();
            event_loop.run();
        })
    }
}

/// Helper trait because `JoinHandle::is_alive` is not stable.
trait ThreadHandleExt {
    fn is_alive(&self) -> bool;
}

impl<T> ThreadHandleExt for JoinHandle<T> {
    fn is_alive(&self) -> bool {
        !self.is_finished()
    }
}
