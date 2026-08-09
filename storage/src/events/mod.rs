use byteser_derive::ByteSerializable;
use std::sync::{Arc, Mutex, OnceLock, Weak};
use std::sync::atomic::{AtomicU64, Ordering};

pub mod loops;
pub mod queue;
pub mod lifecycle;
pub mod dispatcher;
pub mod producer;
pub mod consumer;

pub use loops::EventLoopSupervisor;

static GLOBAL_EVENT_LOOP_SUPERVISOR: OnceLock<EventLoopSupervisor> = OnceLock::new();
static STORAGE_EVENT_SUPERVISOR: OnceLock<Mutex<Option<Weak<EventLoopSupervisor>>>> = OnceLock::new();
static NEXT_TASK_ID: AtomicU64 = AtomicU64::new(1);

fn storage_event_supervisor_registry() -> &'static Mutex<Option<Weak<EventLoopSupervisor>>> {
    STORAGE_EVENT_SUPERVISOR.get_or_init(|| Mutex::new(None))
}

pub fn register_storage_event_supervisor(supervisor: &Arc<EventLoopSupervisor>) {
    *storage_event_supervisor_registry().lock().unwrap() = Some(Arc::downgrade(supervisor));
}

pub fn unregister_storage_event_supervisor() {
    *storage_event_supervisor_registry().lock().unwrap() = None;
}

pub fn current_storage_event_supervisor() -> Option<Arc<EventLoopSupervisor>> {
    storage_event_supervisor_registry()
        .lock()
        .unwrap()
        .as_ref()
        .and_then(|weak| weak.upgrade())
}

fn next_task_id() -> u64 {
    NEXT_TASK_ID.fetch_add(1, Ordering::Relaxed)
}

pub fn publish_storage_event(payload: Vec<u8>) {
    if let Some(supervisor) = current_storage_event_supervisor() {
        let task = Task {
            id: next_task_id(),
            payload,
        };

        if let Err(err) = supervisor.push(task) {
            eprintln!("[storage] failed to publish storage event: {:?}", err);
        }
    }
}

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
