
pub mod loops;
pub mod errors;
pub mod queue;

#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeEvent {
    TaskCompleted,
    TaskFailed,
    QueueOverflow,
    ShutdownRequested,
}
