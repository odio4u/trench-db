use storage::queue::EventLoop;
use std::panic::{self, AssertUnwindSafe};
use std::sync::{Arc, Mutex};

#[test]
fn executes_tasks_in_order() {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let event_loop = EventLoop::new();

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
    let event_loop = EventLoop::new();

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
    let event_loop = EventLoop::new();

    event_loop.post(|| panic!("boom")).unwrap();

    let buffer2 = Arc::clone(&buffer);
    event_loop.post(move || buffer2.lock().unwrap().push("Alive")).unwrap();

    event_loop.run().unwrap();

    let values = buffer.lock().unwrap();
    assert_eq!(values.as_slice(), ["Alive"]);
}

#[test]
fn nested_posting_during_execution_works() {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let event_loop = EventLoop::new();
    let handle = event_loop.handle();
    let buffer_for_task = buffer.clone();

    event_loop.post(move || {
        let nested_buffer = buffer_for_task.clone();
        handle.post(move || {
            nested_buffer.lock().unwrap().push("nested");
        }).unwrap();
    }).unwrap();

    event_loop.run().unwrap();
    let values = buffer.lock().unwrap();
    assert_eq!(values.as_slice(), ["nested"]);
}

#[test]
fn graceful_stop_finishes_remaining_tasks() {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let event_loop = EventLoop::new();

    let buffer1 = Arc::clone(&buffer);
    event_loop.post(move || buffer1.lock().unwrap().push("first")).unwrap();

    let buffer2 = Arc::clone(&buffer);
    event_loop.post(move || buffer2.lock().unwrap().push("second")).unwrap();

    event_loop.stop();
    event_loop.run().unwrap();

    let values = buffer.lock().unwrap();
    assert_eq!(values.as_slice(), ["first", "second"]);
}

#[test]
fn immediate_stop_drops_remaining_tasks() {
    let buffer = Arc::new(Mutex::new(Vec::new()));
    let event_loop = EventLoop::new();
    let handle = event_loop.handle();
    let buffer_for_task = buffer.clone();

    event_loop.post(move || {
        let nested_buffer = buffer_for_task.clone();
        let nested_buffer_dropped = nested_buffer.clone();
        handle.post(move || nested_buffer_dropped.lock().unwrap().push("dropped")).unwrap();

        let nested_buffer_kept = nested_buffer.clone();
        handle.post(move || nested_buffer_kept.lock().unwrap().push("kept")).unwrap();
    }).unwrap();

    event_loop.stop_immediate();
    let result = event_loop.run();

    assert!(matches!(result, Err(_)));
    let values = buffer.lock().unwrap();
    assert!(values.is_empty());
}

#[test]
fn runtime_events_are_emitted_for_task_failures_and_shutdown() {
    let event_loop = EventLoop::new();
    event_loop.post(|| panic!("boom")).unwrap();
    event_loop.post(|| {}).unwrap();
    event_loop.stop();

    event_loop.run().unwrap();
    let events = event_loop.take_events();

    assert!(events.contains(&storage::queue::RuntimeEvent::TaskFailed));
    assert!(events.contains(&storage::queue::RuntimeEvent::TaskCompleted));
    assert!(events.contains(&storage::queue::RuntimeEvent::ShutdownRequested));
}

#[test]
fn stop_prevents_posts_after_stopping() {
    let event_loop = EventLoop::new();
    event_loop.run().unwrap();
    event_loop.stop();
    assert!(matches!(event_loop.post(|| {}), Err(_)));
}

#[test]
fn run_returns_error_when_rerun_after_stop() {
    let event_loop = EventLoop::new();
    event_loop.run().unwrap();
    let result = event_loop.run();
    assert!(matches!(result, Err(_)));
}

#[test]
fn post_fails_when_stop_requested_before_run() {
    let event_loop = EventLoop::new();
    event_loop.stop();
    assert!(matches!(event_loop.post(|| {}), Err(_)));
}

#[test]
fn no_tasks_leak_after_panic() {
    let event_loop = EventLoop::new();

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
