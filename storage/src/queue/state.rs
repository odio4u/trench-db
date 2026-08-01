//! Event loop lifecycle states.

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
