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
                Err(RecvTimeoutError::Timeout) => {
                    continue;
                }
            }
        }

        self.lifecycle.complete_shutdown();
    }

    fn dispatch(&self, task: Task) {
        self.dispatcher.dispatch(task.id, task.payload);
    }
}
