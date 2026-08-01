# Storage Queue Design

This folder documents the storage-layer event loop kernel used by TrenchDB's storage runtime.

## Purpose

The storage queue implements a minimal event loop kernel that is:

- single-threaded and sequential
- simple to understand
- resilient to user callbacks
- extensible through a small, well-defined interface

It is intentionally not a full scheduler, timer system, or networking runtime. Those responsibilities are layered on top of this kernel later.

## Invariants

The storage queue follows these invariants:

- A task executes exactly once.
- Tasks never execute concurrently.
- User code may panic; the runtime catches it and continues.
- Posting a task during execution is supported.
- Stopping is deterministic and controlled through explicit policies.

## Core Responsibilities

The kernel owns exactly four responsibilities:

- Accept new tasks (`post()`)
- Execute tasks in order (`run()`)
- Recover from user failures (`panic_boundary`)
- Shutdown cleanly (`stop()` / `stop_immediate()`)

## Internal Structure

The queue is composed of separate concerns:

- `State` — lifecycle states: `Created`, `Running`, `Stopping`, `Stopped`
- `EventQueue` — a FIFO `VecDeque<Task>` that holds pending work
- `Dispatcher` — executes one task at a time and knows nothing about networking
- `PanicBoundary` — isolates panics from user callbacks and converts them into events
- `Lifecycle` — manages the valid state transitions and stop semantics

## Execution Model

The runtime is intentionally simple. In V1 the core loop is effectively:

```text
while running {
    while let Some(task) = queue.pop() {
        dispatch(task);
    }
}
```

Because there is only one thread, sequential execution is guaranteed.

## Runtime Events

Failures and lifecycle signals are represented as events rather than panics or ad-hoc logging:

- `RuntimeEvent::TaskCompleted`
- `RuntimeEvent::TaskFailed`
- `RuntimeEvent::QueueOverflow`
- `RuntimeEvent::ShutdownRequested`

This event model allows future metrics, tracing, and observers to subscribe without changing the core.

## Stop Policies

Two shutdown modes are supported:

- `stop()` — graceful shutdown: finish remaining queued tasks, then stop
- `stop_immediate()` — immediate shutdown: drop remaining queued tasks

Graceful is the default and is the recommended policy for normal shutdown.

## Extensibility

The kernel is designed to be a foundation, not a complete application runtime. Additional features can be layered on top without changing the core:

- timers
- networking
- async/futures
- sharded scheduling
- admission control

Those features should be built as plugins around this kernel, not inside it.
