# Storage Event Runtime

This directory documents the storage-layer event runtime, formerly known as the `queue` kernel.

## Documents

- [design.md](design.md) — architecture, invariants, and Mermaid diagrams.
- [usage.md](usage.md) — API examples and usage patterns.
- [storage-integration.md](storage-integration.md) — how the event runtime is wired into `MemoryStore` and collection mutations.

## Quick Overview

The event runtime is a thread-safe, bounded, producer/consumer task queue with:

- Multiple producers pushing concurrently.
- A single consumer runner executing tasks sequentially.
- Lifecycle management for graceful/immediate shutdown.
- An `EventLoopSupervisor` that respawns the runner thread if it dies.

## Mermaid Architecture

```mermaid
graph TB
    subgraph Producers["Producers (any thread)"]
        P1[ProducerHandle]
        P2[ProducerHandle]
    end

    subgraph SharedQueue["SharedQueue<T>"]
        M[Mutex<VecDeque<T>>]
        C[Condvar]
        S[Mutex<QueueStats>]
        A[AtomicUsize active_runners]
    end

    subgraph Consumer["Consumer (runner thread)"]
        CH[ConsumerHandle]
        EL[EventLoop]
        D[Dispatcher]
    end

    ELS[EventLoopSupervisor]

    P1 -->|push| M
    P2 -->|push| M
    C -->|notify| CH
    CH -->|recv| M
    EL --> CH
    EL --> D
    ELS -->|spawn / respawn| EL
```
