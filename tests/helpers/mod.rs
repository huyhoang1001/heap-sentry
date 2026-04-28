use heap_sentry::{analysis::{init, init_tracker}, config::{OutputFormat, TrackerConfig}, metrics::METRICS};
use std::sync::{Mutex, OnceLock};

static SCENARIO_LOCK: OnceLock<Mutex<()>> = OnceLock::new();
static INIT_TRACKER: OnceLock<()> = OnceLock::new();
#[allow(dead_code)]
static INIT_AUTO_TRACKER: OnceLock<()> = OnceLock::new();

pub fn scenarios_mutex() -> &'static Mutex<()> {
    SCENARIO_LOCK.get_or_init(|| Mutex::new(()))
}

pub fn setup_tracker() {
    INIT_TRACKER.get_or_init(|| {
        init_tracker(TrackerConfig {
            sampling_interval_ms: 200,
            growth_threshold_bytes_per_sec: 1,
            leak_threshold_bytes: 1,
            enable_backtrace: false,
            backtrace_sample_rate: 100,
            output_format: OutputFormat::Stderr,
        })
        .expect("Failed to initialize tracker for scenario tests");
    });
}

#[allow(dead_code)]
pub fn setup_auto_tracker() {
    INIT_AUTO_TRACKER.get_or_init(|| {
        std::env::set_var("HEAP_SENTRY_BACKTRACE_SAMPLE_RATE", "30");
        init().expect("Failed to initialize tracker via auto config");
        std::env::remove_var("HEAP_SENTRY_BACKTRACE_SAMPLE_RATE");
    });
}

pub fn reset_metrics() {
    METRICS.total_allocated.store(0, std::sync::atomic::Ordering::Relaxed);
    METRICS.total_freed.store(0, std::sync::atomic::Ordering::Relaxed);
    METRICS.current_usage.store(0, std::sync::atomic::Ordering::Relaxed);
    METRICS.allocation_count.store(0, std::sync::atomic::Ordering::Relaxed);
    METRICS.deallocation_count.store(0, std::sync::atomic::Ordering::Relaxed);
    METRICS.active_allocations.lock().unwrap().clear();
    METRICS.stack_traces.lock().unwrap().clear();
    METRICS.allocation_stats.lock().unwrap().clear();
}
