//! Lifecycle operations for the event loop kernel.

use crate::queue::state::State;

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

    pub fn request_stop(&mut self) {
        self.state = match self.state {
            State::Created => State::Stopped,
            State::Running => State::Stopping,
            State::Stopping | State::Stopped => self.state,
        };
    }

    pub fn is_running(&self) -> bool {
        self.state == State::Running
    }

    pub fn is_stopped(&self) -> bool {
        self.state == State::Stopped
    }

    pub fn allows_post(&self) -> bool {
        self.state.allows_post()
    }
}
