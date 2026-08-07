
use std::sync::Arc;
use super::queue::Queue;
use super::queue::SharedQueue;


pub struct ProducerHandle<T> {
    pub(crate) data: Arc<Queue<T>>,
    pub(crate) capacity: usize,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PushError<T> {
    Full(T),
}

impl <T> SharedQueue<T> {
    pub fn producer_handle(&self) -> ProducerHandle<T> {
        ProducerHandle {
            data: Arc::clone(self.inner()),
            capacity: self.capacity(),
        }
    }
}

impl <T> ProducerHandle<T> {

    /// Pushes an item into the queue. If the queue is at capacity, returns
    /// `PushError::Full(item)` instead of growing unbounded.
    pub fn push(&self, item: T) -> Result<(), PushError<T>> {
        let mut queue = self.data.queue.lock().unwrap();

        if self.capacity > 0 && queue.len() >= self.capacity {
            return Err(PushError::Full(item));
        }

        let was_empty = queue.is_empty();
        queue.push_back(item);

        {
            let mut stats = self.data.queue_stats.lock().unwrap();
            stats.total_pushed += 1;
            stats.max_queue_size = stats.max_queue_size.max(queue.len());
        }

        drop(queue); // Release queue lock before notifying

        if was_empty {
            self.data.cvar.notify_one();
        }

        Ok(())
    }

    pub fn is_empty(&self) -> bool {
        let queue = self.data.queue.lock().unwrap();
        queue.is_empty()
    }

    pub fn len(&self) -> usize {
        let queue = self.data.queue.lock().unwrap();
        queue.len()
    }
}