//! The storage-layer event loop kernel.
//!
//! This crate-local module implements a minimal, single-threaded event loop
//! kernel with clear separation of concern across state, queue, dispatch,
//! panic boundaries, and lifecycle management.

pub mod dispatcher;
pub mod event_loop;
pub mod event_queue;
pub mod lifecycle;
pub mod panic_boundary;
pub mod state;

pub use event_loop::EventLoop;
pub use state::State;
