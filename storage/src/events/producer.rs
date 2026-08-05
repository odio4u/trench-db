
use std::sync::Arc;
use super::queue::Queue;
use super::queue::SharedQueue;


pub struct ProducerHandle<T> {
    data: Arc<Queue<T>>,
}

impl <T> SharedQueue<T> {
    pub fn producer_handle(&self) -> ProducerHandle<T> {
        ProducerHandle {
            data: Arc::clone(&self.queue),
        }
    }
}

impl <T> ProducerHandle<T> {

    pub fn push(&self, item: T) {
        let was_empty = {
            let mut queue = self.data.queue.lock().unwrap();
            let was_empty = queue.is_empty();
            queue.push_back(item);
            was_empty
        };

        {
            let mut stats = self.data.queue_stats.lock().unwrap();
            stats.total_pushed += 1;
            stats.max_queue_size = stats.max_queue_size.max(self.data.queue.lock().unwrap().len());
        }

        // Wake up one waiting consumer after releasing all locks.
        if was_empty {
            self.data.cvar.notify_one();
        }
    }

}
