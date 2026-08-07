pub use crate::events::State;

#[derive(Debug, Copy, Clone)]
pub enum StopPolicy {
    Graceful,
    Immediate,
}

pub struct Lifecycle {
    pub state: State,
}

impl Lifecycle {
    pub fn new() -> Self {
        Self {
            state: State::Created,
        }
    }

    pub fn start(&mut self) {
        if self.state == State::Created {
            self.state = State::Running;
        }
    }

    pub fn request_stop(&mut self, policy: StopPolicy) {
        self.state = match (self.state, policy) {
            (State::Created, StopPolicy::Graceful) => State::Stopping,
            (State::Created, StopPolicy::Immediate) => State::Stopped,
            (State::Running, StopPolicy::Graceful) => State::Stopping,
            (State::Running, StopPolicy::Immediate) => State::Stopped,
            (State::Stopping, _) | (State::Stopped, _) => self.state,
        };
    }

    pub fn complete_shutdown(&mut self) {
        if self.state == State::Stopping || self.state == State::Running {
            self.state = State::Stopped;
        }
    }

    pub fn should_continue(&self) -> bool {
        let data = matches!(self.state, State::Running | State::Stopping);
        data
    }

    pub fn is_created(&self) -> bool {
        self.state == State::Created
    }

    pub fn is_running(&self) -> bool {
        self.state == State::Running
    }

    pub fn is_stopped(&self) -> bool {
        self.state == State::Stopped
    }

    pub fn state(&self) -> State {
        self.state
    }

    pub fn allows_post(&self) -> bool {
        self.state.allows_post()
    }
}
