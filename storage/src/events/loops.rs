
use super::dispatcher;

use super::queue::EventQueue;
use std::time::Duration;
pub struct EventLoop {
    handler: EventLoopHandle,
}

pub struct EventLoopHandle {
    queue: EventQueue,
    dispatcher: dispatcher::Dispatcher,
}


impl EventLoop {

    pub fn new() -> Self {
        let queue = EventQueue::new();
        let dispatcher = dispatcher::Dispatcher::new();
        let handler = EventLoopHandle { queue, dispatcher };
        Self { handler }
    }

    pub fn post_task(&mut self, task: super::Task) {
        self.handler.queue.push(task);
    }

    pub fn run(&mut self) {
        loop {
            if self.handler.queue.is_empty() {
                sleep_for(Duration::from_millis(10));
                continue;
            }

            if let Some(task) = self.handler.queue.pop() {
                let mut id = task.id;
                self.handler.dispatcher.dispatch(id, task.payload);
            }
        }
    }
}

pub fn sleep_for(duration: Duration) {
    std::thread::sleep(duration);
}