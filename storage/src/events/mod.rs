use byteser_derive::ByteSerializable;
use std::sync::OnceLock;

pub mod loops;
pub mod queue;
pub mod lifecycle;
pub mod dispatcher;
pub mod producer;
pub mod consumer;

pub use loops::EventLoopSupervisor;

static GLOBAL_EVENT_LOOP_SUPERVISOR: OnceLock<EventLoopSupervisor> = OnceLock::new();

pub fn init_global_event_loop_supervisor(capacity: usize) -> &'static EventLoopSupervisor {
    GLOBAL_EVENT_LOOP_SUPERVISOR.get_or_init(|| {
        let queue = queue::SharedQueue::with_capacity(capacity);
        let supervisor = EventLoopSupervisor::new(queue);
        supervisor.start();
        supervisor
    })
}

pub fn global_event_loop_supervisor() -> &'static EventLoopSupervisor {
    init_global_event_loop_supervisor(usize::MAX)
}

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
