use std::sync::{Arc, Mutex};

pub use crate::events::State;

#[derive(Debug, Copy, Clone)]
pub enum StopPolicy {
    Graceful,
    Immediate,
}

#[derive(Debug, Clone)]
pub struct Lifecycle {
    state: Arc<Mutex<State>>,
}

impl Lifecycle {
    pub fn new() -> Self {
        Self {
            state: Arc::new(Mutex::new(State::Created)),
        }
    }

    pub fn clone(&self) -> Self {
        Self {
            state: Arc::clone(&self.state),
        }
    }

    pub fn start(&self) {
        let mut state = self.state.lock().unwrap();
        if *state == State::Created {
            *state = State::Running;
        }
    }

    pub fn request_stop(&self, policy: StopPolicy) {
        let mut state = self.state.lock().unwrap();
        *state = match (*state, policy) {
            (State::Created, StopPolicy::Graceful) => State::Stopping,
            (State::Created, StopPolicy::Immediate) => State::Stopped,
            (State::Running, StopPolicy::Graceful) => State::Stopping,
            (State::Running, StopPolicy::Immediate) => State::Stopped,
            (State::Stopping, _) | (State::Stopped, _) => *state,
        };
    }

    pub fn complete_shutdown(&self) {
        let mut state = self.state.lock().unwrap();
        if *state == State::Stopping || *state == State::Running {
            *state = State::Stopped;
        }
    }

    pub fn should_continue(&self) -> bool {
        let state = self.state.lock().unwrap();
        matches!(*state, State::Running | State::Stopping)
    }

    pub fn is_created(&self) -> bool {
        let state = self.state.lock().unwrap();
        *state == State::Created
    }

    pub fn is_running(&self) -> bool {
        let state = self.state.lock().unwrap();
        *state == State::Running
    }

    pub fn is_stopped(&self) -> bool {
        let state = self.state.lock().unwrap();
        *state == State::Stopped
    }

    pub fn state(&self) -> State {
        let state = self.state.lock().unwrap();
        *state
    }

    pub fn allows_post(&self) -> bool {
        self.state().allows_post()
    }
}
