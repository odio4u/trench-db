use std::sync::Arc;
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
    pub fn new(queue: &SharedQueue<Task>) -> Self {
        Self {
            lifecycle: Lifecycle::new(),
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
    handle: Option<JoinHandle<()>>,
}

impl EventLoopSupervisor {
    pub fn new(queue: SharedQueue<Task>) -> Self {
        Self {
            shared_queue: Arc::new(queue),
            handle: None,
        }
    }

    pub fn start(&mut self) {
        self.ensure_runner_alive();
    }

    /// Pushes a task into the queue. If the runner thread is no longer alive,
    /// a new one is spawned before returning.
    pub fn push(&mut self, task: Task) -> Result<(), PushError<Task>> {
        self.ensure_runner_alive();

        let producer = self.shared_queue.producer_handle();
        let result = producer.push(task);

        // If the runner died between our check and the push, re-spawn so the
        // message is not left unprocessed indefinitely.
        if self.runner_is_dead() {
            self.spawn_runner();
        }

        result
    }

    pub fn request_stop(&mut self) {
        if let Some(handle) = self.handle.take() {
            let _ = handle.join();
        }
    }

    fn ensure_runner_alive(&mut self) {
        if self.runner_is_dead() {
            self.spawn_runner();
        }
    }

    fn runner_is_dead(&self) -> bool {
        if let Some(handle) = &self.handle {
            !handle.is_alive()
        } else {
            true
        }
    }

    fn spawn_runner(&mut self) {
        let queue = Arc::clone(&self.shared_queue);
        // TODO: Self.shared_queue should be should be handled properly so there won't live any stale queue data in memory.
        let handle = thread::spawn(move || {
            let mut event_loop = EventLoop::new(&queue);
            event_loop.start();
            event_loop.run();
        });

        self.handle = Some(handle);
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
