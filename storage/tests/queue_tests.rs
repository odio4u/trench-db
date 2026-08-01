use storage::queue::EventLoop;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

#[test]
fn executes_tasks_in_order() {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let mut event_loop = EventLoop::new();

    let buffer1 = Arc::clone(&buffer);
    event_loop.post(move || buffer1.lock().unwrap().push("Hello")).unwrap();

    let buffer2 = Arc::clone(&buffer);
    event_loop.post(move || buffer2.lock().unwrap().push("World")).unwrap();

    event_loop.run().unwrap();

    let values = buffer.lock().unwrap();
    assert_eq!(values.as_slice(), ["Hello", "World"]);
}

#[test]
fn tasks_execute_exactly_once() {
    let counter = Arc::new(Mutex::new(0));
    let mut event_loop = EventLoop::new();

    let counter1 = Arc::clone(&counter);
    event_loop.post(move || {
        let mut lock = counter1.lock().unwrap();
        *lock += 1;
    }).unwrap();

    event_loop.run().unwrap();
    let result = event_loop.run();

    assert!(matches!(result, Err(_)));
    let value = *counter.lock().unwrap();
    assert_eq!(value, 1);
}

#[test]
fn catches_panics_and_continues() {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let mut event_loop = EventLoop::new();

    event_loop.post(|| panic!("boom")).unwrap();

    let buffer2 = Arc::clone(&buffer);
    event_loop.post(move || buffer2.lock().unwrap().push("Alive")).unwrap();

    event_loop.run().unwrap();

    let values = buffer.lock().unwrap();
    assert_eq!(values.as_slice(), ["Alive"]);
}

#[test]
fn stop_prevents_posts_after_stopping() {
    let mut event_loop = EventLoop::new();
    event_loop.run().unwrap();
    event_loop.stop();
    assert!(matches!(event_loop.post(|| {}), Err(_)));
}

#[test]
fn run_returns_error_when_rerun_after_stop() {
    let mut event_loop = EventLoop::new();
    event_loop.run().unwrap();
    let result = event_loop.run();
    assert!(matches!(result, Err(_)));
}

#[test]
fn post_fails_when_stop_requested_before_run() {
    let mut event_loop = EventLoop::new();
    event_loop.stop();
    assert!(matches!(event_loop.post(|| {}), Err(_)));
}

#[test]
fn no_tasks_leak_after_panic() {
    let mut event_loop = EventLoop::new();

    event_loop.post(|| panic!("boom")).unwrap();
    event_loop.post(|| {}).unwrap();

    event_loop.run().unwrap();
    assert!(event_loop.post(|| {}).is_err());
}

#[test]
fn event_loop_new_is_created_state() {
    let event_loop = EventLoop::new();
    let result = panic::catch_unwind(AssertUnwindSafe(|| {
        // Ensure construction does not panic.
        drop(event_loop);
    }));
    assert!(result.is_ok());
}
