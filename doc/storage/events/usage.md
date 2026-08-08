# Storage Event Runtime — Usage

This document shows how to use the storage-layer event runtime in TrenchDB.

## Basic supervised event loop

```rust
use storage::events::queue::SharedQueue;
use storage::events::loops::EventLoopSupervisor;
use storage::events::Task;

let queue = SharedQueue::with_capacity(1024);
let mut supervisor = EventLoopSupervisor::new(queue);
supervisor.start();

supervisor.push(Task { id: 1, payload: vec![1, 2, 3] }).unwrap();
```

## Manual event loop (no supervision)

```rust
use storage::events::queue::SharedQueue;
use storage::events::loops::EventLoop;
use storage::events::Task;

let queue = SharedQueue::with_capacity(1024);
let mut event_loop = EventLoop::new(&queue);

let producer = event_loop.producer();
producer.push(Task { id: 1, payload: vec![1, 2, 3] }).unwrap();

event_loop.start();
event_loop.run(); // blocks until stopped
```

## Graceful shutdown

```rust
use storage::events::lifecycle::StopPolicy;

// From the thread that owns the EventLoop:
event_loop.request_stop(StopPolicy::Graceful);
```

With the supervisor, stop is synchronous:

```rust
supervisor.request_stop();
```

## Panic isolation

The current dispatcher simply prints the task. To isolate user panics, wrap dispatching with `std::panic::catch_unwind`:

```rust
use std::panic::{catch_unwind, AssertUnwindSafe};

fn dispatch(&self, task: Task) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        self.dispatcher.dispatch(task.id, task.payload);
    }));
}
```

This prevents a panicking task from killing the runner thread and triggering unnecessary respawns.

## API Summary

### SharedQueue

- `SharedQueue::new()` — create an unbounded queue.
- `SharedQueue::with_capacity(n)` — create a bounded queue.
- `SharedQueue::producer_handle()` — obtain a `ProducerHandle`.
- `SharedQueue::consumer_handle()` — obtain a `ConsumerHandle`.
- `SharedQueue::active_runners()` — number of active runners.

### ProducerHandle

- `push(item)` — push an item; returns `PushError::Full(item)` if at capacity.
- `is_empty()` — true if the queue is empty.
- `len()` — current queue length.

### ConsumerHandle

- `recv()` — block until an item is available.
- `recv_timeout(duration)` — block up to `duration`; returns `RecvTimeoutError::Timeout`.
- `try_recv()` / `pop()` — return an item if one is available, otherwise `None`.

### EventLoop

- `EventLoop::new(queue)` — create a loop bound to `queue`.
- `start()` — transition lifecycle to `Running`.
- `request_stop(policy)` — request graceful or immediate stop.
- `producer()` — clone a `ProducerHandle` for external use.
- `post_task(task)` — push a task if lifecycle allows posting.
- `run()` — run the loop until stopped.

### EventLoopSupervisor

- `EventLoopSupervisor::new(queue)` — create a supervisor.
- `start()` — spawn the initial runner.
- `push(task)` — push a task, respawning the runner if needed.
- `request_stop()` — join the runner and stop supervision.

## Notes

- The queue is thread-safe and can be shared across threads.
- Multiple producers can push concurrently.
- Only one `EventLoop` should consume from a queue unless you intentionally want competing consumers.
- The supervisor checks runner health on every `push` and respawns automatically.
