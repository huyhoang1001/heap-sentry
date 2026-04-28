#[path = "helpers/mod.rs"]
mod common;

use common::{reset_metrics, scenarios_mutex, setup_tracker};
use heap_sentry::{config::TrackerConfig, metrics::{SafeMutex, MetricsError}, snapshot, thread_stats, current_thread_stats};
use std::panic;
use std::thread;
use std::time::Duration;

#[test]
fn config_validation_and_thread_stats_work() {
    assert!(TrackerConfig::default().validate().is_ok());

    let invalid = TrackerConfig {
        sampling_interval_ms: 0,
        ..TrackerConfig::default()
    };
    assert!(invalid.validate().is_err());

    let stats = thread_stats();
    assert_eq!(stats.len(), 1);
    assert_eq!(stats[0].thread_id, "main");

    let current = current_thread_stats();
    assert_eq!(current.thread_id, "current");
}

#[test]
fn safe_mutex_poisoning_recoverable() {
    let mutex = SafeMutex::new(0usize);

    let _ = panic::catch_unwind(|| {
        let mut guard = mutex.lock().unwrap();
        *guard = 123;
        panic!("force poison");
    });

    let result = mutex.lock();
    assert!(matches!(result, Err(MetricsError::MutexPoisoned)));
}

#[test]
fn macro_scope_tracking_works() {
    let _guard = scenarios_mutex().lock().unwrap();
    setup_tracker();
    reset_metrics();

    heap_sentry::track_scope!("macro_scope_test", {
        let _data = vec![0u8; 256 * 1024];
        thread::sleep(Duration::from_millis(20));
    });

    let stats = snapshot();
    assert!(stats.total_allocated >= 256 * 1024, "macro scope allocations should count toward global stats");
}
