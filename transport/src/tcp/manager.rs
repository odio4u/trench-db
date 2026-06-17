// ── State machine ────────────────────────────────────────────────────────────
//
//                   Open
//                  /    \
//   send CLOSE(FIN)      receive CLOSE(FIN)
//        /                        \
// HalfClosedLocal          HalfClosedRemote
//        \                        /
//   receive CLOSE(FIN,ACK)  send CLOSE(FIN)
//         \                      /
//                  Closed
//
//    any state + RESET (either side) → Reset (immediately)

use bytes::Bytes;
use std::collections::VecDeque;


pub const DEFAULT_WINDOW: i64 = 65_536;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum StreamState {
    Open,
    HalfClosedLocal,
    HalfClosedRemote,
    Closed,
    Reset,
}

impl StreamState {
    pub fn can_send(&self) -> bool {
        matches!(self, StreamState::Open | StreamState::HalfClosedRemote)
    }
    pub fn can_receive(&self) -> bool {
        matches!(self, StreamState::Open | StreamState::HalfClosedLocal)
    }
    pub fn is_closed(&self) -> bool {
        matches!(self, StreamState::Closed | StreamState::Reset)
    }
}


pub struct Stream {
    pub id: u32,
    pub state: StreamState,
    pub send_window: i64,
    pub recv_window: i64, 
    recv_queue: VecDeque<Bytes>,
}
