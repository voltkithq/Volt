use super::*;
use std::time::Instant;

#[test]
fn gc_scheduler_runs_after_request_threshold() {
    let mut scheduler = worker::GcScheduler::new();
    scheduler.requests_since_gc = JS_GC_REQUEST_INTERVAL - 1;
    assert!(!scheduler.should_run_gc());

    scheduler.note_request_completed();
    assert!(scheduler.should_run_gc());
}

#[test]
fn gc_scheduler_runs_after_time_interval() {
    let mut scheduler = worker::GcScheduler::new();
    scheduler.last_gc_at = Instant::now() - JS_GC_INTERVAL - Duration::from_millis(1);
    assert!(scheduler.should_run_gc());
}

#[test]
fn gc_scheduler_reset_after_gc_run() {
    let mut scheduler = worker::GcScheduler::new();
    scheduler.requests_since_gc = JS_GC_REQUEST_INTERVAL;
    assert!(scheduler.should_run_gc());

    scheduler.mark_gc_run();
    assert_eq!(scheduler.requests_since_gc, 0);
    assert!(!scheduler.should_run_gc());
}
