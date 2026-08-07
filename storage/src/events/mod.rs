use byteser_derive::ByteSerializable;



pub mod loops;
pub mod queue;
pub mod lifecycle;
pub mod dispatcher;
pub mod producer;
pub mod consumer;

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



#[derive(Debug, Copy, Clone, PartialEq, Eq)]
pub enum State {
    Created,
    Running,
    Stopping,
    Stopped,
}

impl State {
    pub fn allows_post(self) -> bool {
        let data = matches!(self, State::Created | State::Running);
        data
    }
}
