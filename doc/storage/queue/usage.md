# Storage Queue Usage

This document shows how to use the storage-layer event loop kernel in TrenchDB.

## Basic example

```rust
use storage::EventLoop;

let event_loop = EventLoop::new();

event_loop.post(|| println!("Hello")).unwrap();
event_loop.post(|| println!("World")).unwrap();

event_loop.run().unwrap();
```

Output:

```text
Hello
World
```

## Posting nested tasks

Posting from inside a running task is supported through `EventLoopHandle`.

```rust
use storage::EventLoop;
use std::sync::{Arc, Mutex};

let event_loop = EventLoop::new();
let handle = event_loop.handle();
let buffer = Arc::new(Mutex::new(Vec::new()));
let buffer2 = buffer.clone();

event_loop.post(move || {
    let nested_buffer = buffer2.clone();
    handle.post(move || nested_buffer.lock().unwrap().push("nested")).unwrap();
}).unwrap();

event_loop.run().unwrap();

assert_eq!(buffer.lock().unwrap().as_slice(), ["nested"]);
```

## Panic isolation

If a posted task panics, the runtime catches it and continues executing subsequent tasks.

```rust
use storage::EventLoop;

let event_loop = EventLoop::new();
event_loop.post(|| panic!("oops")).unwrap();
event_loop.post(|| println!("Still alive")).unwrap();
event_loop.run().unwrap();
```

The runtime does not crash, and the second task still executes.

## Shutdown policies

### Graceful stop

The default `stop()` behavior finishes queued tasks before stopping:

```rust
let event_loop = EventLoop::new();
event_loop.post(|| println!("task" )).unwrap();
event_loop.stop();
event_loop.run().unwrap();
```

### Immediate stop

If you want to drop remaining work immediately:

```rust
let event_loop = EventLoop::new();
event_loop.post(|| println!("task" )).unwrap();
event_loop.stop_immediate();
let result = event_loop.run();
```

## Event inspection

The runtime emits a small event stream that can be consumed after `run()`.

```rust
use storage::queue::RuntimeEvent;

let events = event_loop.take_events();
assert!(events.contains(&RuntimeEvent::TaskCompleted));
```

## API summary

- `EventLoop::new()` — create a new kernel
- `event_loop.post(task)` — queue a task
- `event_loop.handle()` — obtain a cloneable handle for nested posting
- `event_loop.run()` — execute queued tasks until shutdown
- `event_loop.stop()` — request graceful shutdown
- `event_loop.stop_immediate()` — request immediate shutdown
- `event_loop.take_events()` — retrieve runtime events emitted during execution

## Notes

- The queue is currently single-threaded.
- Tasks are executed in FIFO order.
- Task execution is isolated from panics.
- This kernel is intentionally small and meant for building higher-level runtime features.
