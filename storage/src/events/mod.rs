
pub mod loops;
pub mod errors;
pub mod queue;
pub mod dispatcher;

#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeEvent {
    TaskCompleted,
    TaskFailed,
    QueueOverflow,
    ShutdownRequested,
}
