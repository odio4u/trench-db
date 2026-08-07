
use std::sync::Arc;
use std::time::Duration;
use std::collections::VecDeque;
use super::queue::Queue;
use super::queue::SharedQueue;


pub struct ConsumerHandle<T> {
   data: Arc<Queue<T>>,
}


#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum RecvTimeoutError {
    Timeout,
}

impl <T> SharedQueue<T> {
    pub fn consumer_handle(&self) -> ConsumerHandle<T> {
        ConsumerHandle {
            data: Arc::clone(self.inner()),
        }
    }
}

impl<T> ConsumerHandle<T> {
    pub fn recv(&self) -> T {
        let mut queue = self.data.queue.lock().unwrap();
        loop {
            if let Some(item) = queue.pop_front() {
                self.record_pop(&mut queue);
                return item;
            }
            queue = self.data.cvar.wait(queue).unwrap();
        }
    }

    /// Blocking receive with a timeout. Returns `Err(RecvTimeoutError::Timeout)`
    /// if no item becomes available within `timeout`.
    pub fn recv_timeout(&self, timeout: Duration) -> Result<T, RecvTimeoutError> {
        let mut queue = self.data.queue.lock().unwrap();
        loop {
            if let Some(item) = queue.pop_front() {
                self.record_pop(&mut queue);
                return Ok(item);
            }

            let result = self.data.cvar.wait_timeout(queue, timeout).unwrap();
            queue = result.0;
            if result.1.timed_out() {
                return Err(RecvTimeoutError::Timeout);
            }
        }
    }

    pub fn try_recv(&self) -> Option<T> {
        let mut queue = self.data.queue.lock().unwrap();
        if let Some(item) = queue.pop_front() {
            self.record_pop(&mut queue);
            Some(item)
        } else {
            None
        }
    }

    pub fn pop(&self) -> Option<T> {
        self.try_recv()
    }

    fn record_pop(&self, queue: &mut std::sync::MutexGuard<VecDeque<T>>) {
        let mut stats = self.data.queue_stats.lock().unwrap();
        stats.total_popped += 1;
        let _ = queue.len(); // keep borrow alive if needed, compiler will elide
    }
}
