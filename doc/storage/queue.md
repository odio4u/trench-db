# Production-Grade Event Loop Runtime — Architecture

## 0. What changes going from "small runtime" to "production"

The v2 document was single-core, single-threaded-core, and optimized for conceptual clarity. Production changes the constraints:

| Dimension | Small runtime | Production |
|---|---|---|
| Cores used | 1 | N (all available cores) |
| Concurrency model | One EventLoop | One EventLoop **per core (shard)**, no shared mutable state in the hot path |
| I/O backend | epoll/kqueue via `poll()` | io_uring (Linux) with epoll fallback; IOCP (Windows) |
| Memory | "minimal allocations" as an aspiration | Slab/arena allocators, zero-copy buffers, pre-sized pools — allocation in the hot path is a bug, not a style choice |
| Failure handling | catch panics, log | Panic isolation + watchdog + circuit breakers + load shedding |
| Backpressure | none specified | Bounded queues everywhere, explicit admission control |
| Observability | none specified | Metrics, tracing, structured logs as first-class subsystems |
| Testing | unspecified | Deterministic simulation testing + fuzzing + chaos testing, because concurrency bugs don't reproduce from logs |
| Operability | `run()` / `stop()` | Hot config reload, graceful drain across a fleet, capacity planning model |

This document assumes Linux as the primary target (io_uring), with an abstraction layer for portability. That's a deliberate choice — trying to design an abstraction that's equally optimal on Linux/Windows/macOS from day one produces a worse Linux implementation for a portability need most production deployments don't have. Build for Linux, keep the Poller trait swappable, treat other platforms as a secondary backend.

---

## 1. Concurrency Architecture: Thread-Per-Core (Shard) Model

**Decision: thread-per-core sharding, not a shared work-stealing pool as the default.**

This is the single biggest architectural fork, so it's worth stating the reasoning rather than just the answer.

### The two real options

**A. Shared scheduler + work-stealing (Tokio-style).** One global (or per-worker) task queue; any worker thread can run any task; idle workers steal from busy ones. Good general-purpose throughput, handles uneven load automatically.

**B. Thread-per-core, no cross-core work migration by default (Seastar/Glommio-style).** Each core runs its own fully independent EventLoop, own Poller instance, own Timer Manager, own Scheduler, own memory pools. A connection/task is pinned to whichever core accepted it and never migrates.

### Why B for this runtime

Your original invariants already lean this way — "a callback executes on at most one thread," "deterministic scheduling," "minimal allocations during steady state." Work-stealing directly fights all three:

- Stealing requires cross-core synchronization (at minimum a CAS on a deque), which is exactly the lock/atomic-contention overhead your non-goals list explicitly rejected ("lock-free scheduling" as a non-goal, not a goal — meaning: don't build machinery whose whole purpose is enabling safe cross-core stealing).
- Cross-core migration means a task's memory (buffers, callback state) may end up accessed from a different NUMA node than it was allocated on — a well-known cause of tail latency cliffs at scale.
- Per-core independence gives you actual deterministic scheduling per shard, which you can reason about and test in isolation. A stolen task breaks that determinism.

The cost of thread-per-core is uneven load: if connections aren't distributed evenly across cores, one shard can be hot while others idle. This is solved at admission, not at runtime — see §6 (Load Distribution), not by adding stealing back in.

### Shard structure

```text
                    ┌─────────────────────────────┐
                    │         Supervisor            │  (1 thread — control plane only)
                    │  - health checks per shard     │
                    │  - config distribution         │
                    │  - metrics aggregation          │
                    │  - crash/restart policy          │
                    └───────────────┬─────────────┘
                                    │
        ┌───────────────┬───────────────┬───────────────┐
        │               │               │               │
   ┌────▼────┐     ┌────▼────┐     ┌────▼────┐     ┌────▼────┐
   │ Shard 0  │     │ Shard 1  │     │ Shard 2  │     │ Shard N  │
   │ pinned   │     │ pinned   │     │ pinned   │     │ pinned   │
   │ to CPU 0 │     │ to CPU 1 │     │ to CPU 2 │     │ to CPU N │
   │          │     │          │     │          │     │          │
   │ EventLoop│     │ EventLoop│     │ EventLoop│     │ EventLoop│
   │ Scheduler│     │ Scheduler│     │ Scheduler│     │ Scheduler│
   │ Poller   │     │ Poller   │     │ Poller   │     │ Poller   │
   │ (io_uring)│     │ (io_uring)│     │ (io_uring)│     │ (io_uring)│
   │ TimerMgr │     │ TimerMgr │     │ TimerMgr │     │ TimerMgr │
   │ Registry │     │ Registry │     │ Registry │     │ Registry │
   │ MemPool  │     │ MemPool  │     │ MemPool  │     │ MemPool  │
   └────┬─────┘     └────┬─────┘     └────┬─────┘     └────┬─────┘
        │                │                │                │
        └───── cross-shard channel (SPSC ring, for the rare cases │
               that genuinely need it — see §5) ─────────────┘
```

Each shard is pinned via `sched_setaffinity` (and ideally isolated from the general scheduler via `isolcpus`/`nohz_full` on latency-critical deployments). Each shard owns its own memory arena — no shared heap in the hot path.

### Cross-shard communication (the exception, not the rule)

Some things genuinely need to cross shards: a shared cache invalidation, a pub/sub fanout, a control-plane command. Use **per-pair SPSC ring buffers** (one Shard 0→1 ring, one Shard 1→0 ring, etc.) rather than a shared MPMC queue — SPSC rings need no CAS, only memory fences, and scale without contention as core count grows. Cross-shard messages are drained once per EventLoop cycle, same slot as the CompletionQueue drain in the v2 design — this is the same pattern, just generalized to "any external-to-this-shard source of work."

---

## 2. I/O Backend: io_uring as Primary

**Decision: io_uring on Linux ≥5.11 (ideally ≥5.19 for reliable multi-shot support), epoll fallback for older kernels/containers with restricted io_uring access.**

### Why io_uring over epoll for production

- **True async, not readiness-based.** epoll tells you "this fd is readable," then you still do a syscall to read. io_uring lets you submit the *read itself* and get notified on completion — this eliminates a syscall per operation, which matters enormously at high connection counts.
- **Batched submission/completion.** One `io_uring_enter` can submit hundreds of operations and reap hundreds of completions. epoll's `epoll_wait` only reaps; every operation still costs its own syscall.
- **Multi-shot operations.** A single `accept` or `recv` submission can produce repeated completions without resubmitting each time (kernel ≥5.19), cutting submission overhead further for high-throughput accept loops.

### Structure

```rust
struct ShardPoller {
    ring: io_uring::IoUring,          // one ring per shard, never shared
    sq_capacity: u32,                  // sized to expected in-flight ops per shard
    inflight: SlotMap<OpToken, PendingOp>,
}

enum PendingOp {
    Accept { listener_fd: RawFd },
    Read   { fd: RawFd, buf: PooledBuffer },
    Write  { fd: RawFd, buf: PooledBuffer },
    Timeout,                            // io_uring can also carry timer completions natively
}
```

Key production details epoll-based designs skip:

- **Submission queue is bounded.** If `inflight` exceeds `sq_capacity`, new I/O requests are queued in the shard's own backpressure queue rather than silently blocking — see §6.
- **Buffers are borrowed from the pool for the lifetime of the kernel operation**, not allocated per-op. io_uring's fixed-buffer registration (`IORING_REGISTER_BUFFERS`) lets you register a pool once and reference buffers by index — avoiding both allocation and the page-pinning cost on every I/O call.
- **Poll timeout is native**, not computed via a userspace timer: io_uring supports `IORING_OP_TIMEOUT` as a submission itself, so "wake me by deadline X" is expressed as just another queue entry rather than a special-cased wait parameter. This subsumes the v2 "compute poll timeout from next_deadline()" logic — the timer becomes an io_uring op, not a separate wait computation.

### Fallback path

Detect io_uring availability at startup (`io_uring_setup` probe; also account for seccomp-restricted environments — many container runtimes and some cloud sandboxes disable or limit io_uring). Fall back to epoll + explicit read/write syscalls, keeping the same `Poller` trait surface so the rest of the shard is unaffected. Log which backend is active — this is operationally important, since io_uring and epoll paths have different failure and performance characteristics, and an incident investigation needs to know which one is live.

---

## 3. Memory Management

**Decision: per-shard arenas + size-classed slab pools; no general-purpose allocator calls in the hot path.**

- **Callback/task state**: allocated from a per-shard slab allocator with fixed size classes (e.g., 64B/256B/1KB/4KB slots). A task that needs more than the largest class falls back to the general allocator but this path is logged/counted as a metric — it should be rare, and a rising rate is itself a signal something's using the runtime wrong.
- **I/O buffers**: a registered pool per shard (as noted above for io_uring fixed buffers). Buffers are checked out, used, and returned — never freed and reallocated per operation. Pool sizing is a capacity-planning parameter (§9), not a runtime constant.
- **No cross-shard frees.** A buffer allocated on shard 2's arena is returned to shard 2's pool, even if it was logically "used" elsewhere — enforce this at the type level (buffer handles carry a shard-id, and returning to the wrong shard's pool is a debug-assertion failure, not silently accepted).
- **NUMA awareness**: on multi-socket hosts, pin each shard's arena allocation (`mmap` with `MPOL_BIND` or `numa_alloc_onnode`) to the NUMA node matching that shard's CPU. Cross-node memory access under load is a well-documented source of P99 latency cliffs that don't show up until you're past a few dozen cores.

---

## 4. Scheduler: Per-Shard, Weighted Fair, Aging

Same conceptual tiers as v2 (High/Normal/Low), refined for production load:

- **Weighted round-robin instead of strict priority draining.** Strict "always drain High first" (even bounded) still means a sustained flood of high-priority work can push normal/low into unbounded latency growth under real adversarial or bursty load. Production systems typically use a **weighted fair queueing** discipline: each tier gets a share of each scheduling round proportional to its weight (e.g., 70/25/5), not "all of High, then all of Normal."
- **Priority aging is mandatory, not optional**, in production — a callback's effective weight increases monotonically with wait time, with a hard ceiling so an aged low-priority task eventually preempts sustained high-priority flooding. This is the standard fix for the starvation-under-adversarial-load case that pure fixed-quota systems miss.
- **Admission control sits in front of the scheduler, not behind it** — see §6. By the time work reaches the Scheduler, it has already been admitted; the Scheduler's job is fairness among admitted work, not deciding what gets in.

---

## 5. Timer Manager: Hierarchical Timing Wheel, io_uring-Native Where Possible

Per-shard hierarchical timing wheel (as in v2), with one production addition: where the I/O backend is io_uring, prefer expressing timers as native `IORING_OP_TIMEOUT`/`IORING_OP_LINK_TIMEOUT` submissions tied to the operation they bound (e.g., "time out this read after 500ms") rather than a fully separate software timer wheel running in parallel. This removes an entire class of bugs where a software timer and a kernel-side I/O completion race each other. Reserve the userspace timing wheel for timers not attached to a specific I/O operation (e.g., a periodic housekeeping tick).

---

## 6. Backpressure, Admission Control, and Load Shedding

This entire subsystem is missing from the toy design and is non-negotiable in production.

```text
New connection / request arrives
        │
        ▼
Admission Controller (per shard)
        │
   ┌────┴─────────────────────────┐
   │ Check: shard queue depth,      │
   │ memory pool utilization,        │
   │ in-flight io_uring ops < cap    │
   └────┬─────────────────────────┘
        │
   ┌────┴────┐
 accept     reject/shed
   │           │
   ▼           ▼
Scheduler   Return backpressure signal
             (e.g., TCP-level: stop accepting;
              RPC-level: fast-fail with retry-after)
```

- **Every queue in the system is bounded** — Scheduler tiers, io_uring submission tracking, cross-shard SPSC rings, CompletionQueue equivalent. An unbounded queue anywhere is a memory-exhaustion incident waiting to happen under sustained overload.
- **Load shedding must be cheap.** Rejecting a request should cost less than accepting one — reject at the earliest possible point (e.g., refuse the `accept()` itself, or respond with a pre-serialized "overloaded" response) rather than admitting work partway through and failing later.
- **Shed decisions should be visible**, not silent — every shed increments a metric and, ideally, is sampled into logs/traces so an operator can distinguish "the fleet is legitimately saturated" from "one shard has a bug making it look saturated."

---

## 7. Failure Containment

Building on v2's dispatch-boundary catch, production needs layered containment:

1. **Per-callback isolation** (as in v2): catch panics/exceptions at dispatch, emit `CallbackFailed`, continue.
2. **Per-shard watchdog**: the Supervisor thread expects a heartbeat from each shard every cycle-budget window (e.g., "shard must complete a full cycle within 100ms under normal load"). A shard that misses N consecutive heartbeats is presumed wedged (e.g., a callback that isn't actually panicking but is spinning or blocked on something it shouldn't be) and is either forcibly restarted (dropping its in-flight work, which upstream must be able to tolerate — idempotency/retry is a client-side contract, not something the runtime can invent for you) or flagged for draining.
3. **Circuit breakers at the boundary to external dependencies** (DB calls, downstream RPCs triggered from callbacks): if a dependency's error/timeout rate crosses a threshold, stop sending new work to it for a cooldown window rather than letting every shard queue up retries against a dependency that's already failing — this is what prevents a single slow dependency from backing up every shard's Scheduler.
4. **Bulkheading**: if the runtime hosts multiple logical workloads (e.g., different tenants or different task classes) on the same shard set, cap each workload's share of each shard's scheduling weight so one tenant's flood can't starve another's, tying back into the weighted fair queueing in §4.

---

## 8. Observability (first-class subsystem, not an afterthought)

- **Metrics** (per shard, aggregated at the Supervisor): scheduler queue depth per tier, per-tier wait-time histograms, io_uring submission/completion queue depth, buffer pool utilization, callback execution duration histograms, shed count, watchdog heartbeat latency. Export via a pull endpoint (Prometheus-style) read by the Supervisor thread only — metrics collection must never touch shard hot-path state directly.
- **Tracing**: each task/callback carries a lightweight span id set at admission; spans propagate through re-scheduling (including across the completion-queue/cross-shard-ring boundary) so a request's full path through the runtime is reconstructable — critical for diagnosing tail latency that's otherwise invisible in aggregate metrics.
- **Structured logging**: every recoverable/unrecoverable error, shed decision, and watchdog action logged with shard id, span id, and timestamp — free-text logs are close to useless once you're debugging a specific shard under load at 3am.
- **A shard-local diagnostic snapshot** callable on demand (e.g., via signal or admin socket): current queue depths, oldest-waiting task age per tier, in-flight io_uring op count. This is what you actually reach for during an incident, before metrics have scraped the anomaly.

---

## 9. Testing Strategy (this is where toy vs. production diverges the most)

Concurrency and I/O-backend bugs mostly don't reproduce from a stack trace. Production runtimes need:

- **Deterministic simulation testing.** Run the entire shard logic against a *simulated* clock and *simulated* I/O backend (no real sockets, no real epoll/io_uring) so the same random seed reproduces the exact same interleaving every time. This is how FoundationDB and (in the Rust ecosystem) Turmoil/madsim catch scheduling and timing bugs that are otherwise nearly impossible to reproduce. Build the `Poller` trait abstraction from day one specifically so a `SimulatedPoller` is a drop-in.
- **Fault injection in the simulation layer**: simulated I/O errors, simulated slow completions, simulated partial reads/writes, simulated clock skew between shards — inject these systematically, not just "happy path plus a few manually-written error tests."
- **Concurrency testing for the CompletionQueue/cross-shard rings specifically** (the only genuinely concurrent code in the system): use a tool like Loom (Rust) to exhaustively explore interleavings of the SPSC/MPSC boundary rather than relying on stress-testing and hoping.
- **Chaos/load testing in a staging environment**: kill shards, saturate memory pools, throttle network, and confirm the admission control and watchdog behavior actually degrades gracefully rather than cascading.
- **Fuzzing the parsing/deserialization paths** that sit in front of the scheduler (wherever untrusted bytes turn into tasks) — this is a standard requirement for anything network-facing and is easy to omit from a "runtime" scope if you're thinking of the scheduler as the whole system.

---

## 10. Operability

- **Hot config reload**: batch sizes, weight ratios, admission thresholds, pool sizes should be adjustable without a restart — pushed from the Supervisor to each shard via the same cross-shard control channel used for other signaling, applied at a safe point in the shard's cycle (never mid-batch).
- **Graceful drain across a fleet, not just a process**: `stop(Graceful)` per shard (from v2 §13) composes into a fleet-level drain — stop admitting new work fleet-wide, let in-flight work complete, deregister, exit — coordinated by whatever orchestration layer sits above individual processes (e.g., readiness-probe flip before SIGTERM).
- **Capacity planning as a documented model, not folklore**: given target P99 latency and expected task mix, derive shard count, buffer pool size, and admission thresholds from measured per-task cost (from the histograms in §8) rather than picking round numbers. This should be a spreadsheet/tool, not tribal knowledge held by whoever built it.

---

## 11. Summary: what actually changed vs. the small-runtime design

| Small runtime | Production |
|---|---|
| 1 EventLoop | N sharded EventLoops, thread-per-core, pinned |
| epoll/kqueue via generic Poller | io_uring primary, epoll fallback, kernel-native timers |
| Allocate as needed | Per-shard slab/arena pools, NUMA-pinned, zero-copy buffers |
| Fixed-quota priority batches | Weighted fair queueing + mandatory aging |
| CompletionQueue for thread-pool results | Generalized to per-pair SPSC cross-shard rings |
| Catch panics at dispatch | + per-shard watchdog, circuit breakers, bulkheading |
| (none) | Metrics, tracing, structured logs, live diagnostic snapshot |
| Manual/unit tests | Deterministic simulation testing, fault injection, Loom, chaos testing, fuzzing |
| `run()`/`stop()` | Hot reload, fleet-level graceful drain, capacity planning model |