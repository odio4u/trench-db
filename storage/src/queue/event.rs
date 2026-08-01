//! Core runtime events emitted by the storage event loop.

/// Immutable runtime events produced by the event loop kernel.
#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeEvent {
    TaskCompleted,
    TaskFailed,
    QueueOverflow,
    ShutdownRequested,
}
