use heap_sentry::metrics::{AllocationMeta, MAX_STACK_TRACES, METRICS};
use std::sync::atomic::Ordering;

fn reset_metrics() {
    METRICS.total_allocated.store(0, Ordering::Relaxed);
    METRICS.total_freed.store(0, Ordering::Relaxed);
    METRICS.current_usage.store(0, Ordering::Relaxed);
    METRICS.allocation_count.store(0, Ordering::Relaxed);
    METRICS.deallocation_count.store(0, Ordering::Relaxed);
    METRICS.active_allocations.lock().unwrap().clear();
    METRICS.stack_traces.lock().unwrap().clear();
    METRICS.allocation_stats.lock().unwrap().clear();
}

#[test]
fn stack_trace_storage_is_bounded() {
    reset_metrics();

    for i in 0..(MAX_STACK_TRACES + 10) {
        let stack_id = METRICS.record_stack_trace(format!("trace-{}", i));
        if i < MAX_STACK_TRACES {
            assert_ne!(stack_id, 0, "expected unique stack traces to be tracked when under capacity");
        } else {
            assert_eq!(stack_id, 0, "expected excess stack traces to be rejected when at capacity");
        }
    }

    assert_eq!(METRICS.stack_traces.lock().unwrap().len(), MAX_STACK_TRACES);
}

#[test]
fn only_sampled_allocations_are_stored() {
    reset_metrics();

    METRICS.store_allocation_metadata(0x1000, AllocationMeta { size: 128, stack_id: 0 });
    assert!(METRICS.active_allocations.lock().unwrap().is_empty());

    METRICS.store_allocation_metadata(0x1000, AllocationMeta { size: 128, stack_id: 42 });
    assert_eq!(METRICS.active_allocations.lock().unwrap().len(), 1);
}

#[test]
fn allocation_and_deallocation_update_live_stats() {
    reset_metrics();

    let stack_id = 123;
    METRICS.record_stack_trace("test-stack".to_string());
    METRICS.record_allocation(256, stack_id);
    METRICS.record_deallocation(256, stack_id);

    let stats = METRICS.allocation_stats.lock().unwrap();
    let allocation_stat = stats.get(&stack_id).expect("expected aggregation for stack id");
    assert_eq!(allocation_stat.live_bytes, 0);
    assert_eq!(allocation_stat.alloc_count, 1);
    assert_eq!(allocation_stat.dealloc_count, 1);
}
