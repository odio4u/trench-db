use byteser_derive::ByteSerializable;



pub mod loops;
pub mod queue;
pub mod lifecycle;
pub mod dispatcher;

#[derive(Debug, PartialEq, Eq)]
pub enum RuntimeEvent {
    TaskCompleted,
    TaskFailed,
    QueueOverflow,
    ShutdownRequested,
}

#[derive(Debug, ByteSerializable)]
pub struct  Task {
    pub id: u64,
    pub payload: Vec<u8>,
}