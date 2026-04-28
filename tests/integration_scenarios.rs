#[path = "helpers/mod.rs"]
mod common;

use common::{reset_metrics, scenarios_mutex, setup_auto_tracker, setup_tracker};
use heap_sentry::MemoryScope;
use std::thread;
use std::time::Duration;

#[test]
fn real_world_leak_growth_scenario() {
    let _guard = scenarios_mutex().lock().unwrap();
    setup_tracker();
    reset_metrics();

    let mut leaked = Vec::new();
    for _ in 0..10 {
        leaked.push(vec![0u8; 128 * 1024]);
        thread::sleep(Duration::from_millis(20));
    }

    thread::sleep(Duration::from_secs(1));
    let stats = heap_sentry::snapshot();

    assert!(stats.current_usage >= 10 * 128 * 1024, "expected retained allocations to increase current usage");
    assert!(stats.total_allocated >= stats.total_freed, "expected total allocated to be at least total freed");
}

#[test]
fn real_world_auto_init_scenario() {
    let _guard = scenarios_mutex().lock().unwrap();
    setup_auto_tracker();
    reset_metrics();

    let mut leaked = Vec::new();
    for _ in 0..8 {
        leaked.push(vec![0u8; 100 * 1024]);
        thread::sleep(Duration::from_millis(25));
    }

    thread::sleep(Duration::from_secs(1));
    let stats = heap_sentry::snapshot();

    assert!(stats.current_usage >= 8 * 100 * 1024, "expected auto-init tracker to observe retained memory");
    assert!(stats.total_allocated >= stats.total_freed, "expected total allocations to exceed or equal frees after leak-like workload");
}

#[test]
fn real_world_repeated_alloc_free_scenario() {
    let _guard = scenarios_mutex().lock().unwrap();
    setup_tracker();
    reset_metrics();

    for _ in 0..40 {
        let mut buffer = Vec::with_capacity(32 * 1024);
        buffer.resize(32 * 1024, 1);
        thread::sleep(Duration::from_millis(10));
        drop(buffer);
    }

    thread::sleep(Duration::from_secs(1));
    let stats = heap_sentry::snapshot();

    assert!(stats.total_allocated >= 40 * 32 * 1024, "expected repeated alloc/free activity to be tracked");
    assert!(stats.total_freed >= 40 * 32 * 1024, "expected repeated frees to be tracked");
}

#[test]
fn real_world_nested_scope_scenario() {
    let _guard = scenarios_mutex().lock().unwrap();
    setup_tracker();
    reset_metrics();

    let outer_stats = {
        let outer = MemoryScope::new("outer_scope");
        let inner_stats = {
            let _inner = MemoryScope::new("inner_scope");
            let _data = vec![0u8; 1 * 1024 * 1024];
            thread::sleep(Duration::from_millis(50));
            _inner.stats()
        };

        assert!(inner_stats.allocated >= 1 * 1024 * 1024, "inner scope should capture its own allocations");

        let _data = vec![0u8; 512 * 1024];
        thread::sleep(Duration::from_millis(50));
        outer.stats()
    };

    assert!(outer_stats.allocated >= 1_500_000, "outer scope should include nested and local allocations");
    assert!(outer_stats.allocation_count >= 2, "outer scope should count multiple allocations");
}
