use std::alloc::{GlobalAlloc, Layout, System};
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::metrics::{AllocationMeta, METRICS};

#[cfg(feature = "backtrace")]
use backtrace::Backtrace;

thread_local! {
    static IN_TRACKING: Cell<bool> = Cell::new(false);
}

pub static ENABLE_BACKTRACE: AtomicUsize = AtomicUsize::new(0);
pub static BACKTRACE_SAMPLE_RATE: AtomicUsize = AtomicUsize::new(100);

// Backpressure state

fn is_at_capacity() -> bool {
    METRICS.active_allocation_count.load(Ordering::Relaxed) >= crate::metrics::MAX_TRACKED_ALLOCATIONS / 2
}

fn should_sample_allocation() -> bool {
    // First check if we're at capacity - if so, don't track
    if is_at_capacity() {
        return false;
    }

    let sample_rate = BACKTRACE_SAMPLE_RATE.load(Ordering::Relaxed);
    if sample_rate == 0 {
        return false;
    }

    static SAMPLE_COUNTER: AtomicUsize = AtomicUsize::new(0);
    let index = SAMPLE_COUNTER.fetch_add(1, Ordering::Relaxed);
    index % sample_rate == 0
}

fn capture_stack_id() -> u64 {
    #[cfg(feature = "backtrace")]
    {
        if ENABLE_BACKTRACE.load(Ordering::Relaxed) != 1 {
            return 0;
        }

        let bt = Backtrace::new();
        let key = format!("{:?}", bt);
        METRICS.record_stack_trace(key)
    }

    #[cfg(not(feature = "backtrace"))]
    {
        0
    }
}

/// Generic allocator wrapper that tracks allocations for any underlying allocator
pub struct TrackingAllocator<A: GlobalAlloc> {
    inner: A,
}

impl<A: GlobalAlloc> TrackingAllocator<A> {
    /// Create a new tracking allocator wrapping the given allocator
    pub const fn new(inner: A) -> Self {
        Self { inner }
    }

    /// Track an allocation and record metadata for the pointer
    fn track_alloc(&self, ptr: *mut u8, size: usize) {
        METRICS.record_global_allocation(size);
        if !should_sample_allocation() {
            return;
        }

        IN_TRACKING.with(|active| {
            if active.get() {
                return;
            }
            active.set(true);

            let stack_id = capture_stack_id();
            let meta = AllocationMeta { size, stack_id };
            let stored = METRICS.store_allocation_metadata(ptr as usize, meta);
            if stored {
                METRICS.record_sampled_allocation(size, stack_id);
            }

            active.set(false);
        });
    }

    /// Track a deallocation and update aggregate statistics
    fn track_dealloc(&self, ptr: *mut u8, size: usize) {
        METRICS.record_global_deallocation(size);
        IN_TRACKING.with(|active| {
            if active.get() {
                return;
            }
            active.set(true);

            if let Some(metadata) = METRICS.take_allocation_metadata(ptr as usize) {
                METRICS.record_sampled_deallocation(metadata.size, metadata.stack_id);
            }

            active.set(false);
        });
    }

    /// Track a realloc operation as a deallocation of the old pointer and allocation of the new pointer
    fn track_realloc(&self, old_ptr: *mut u8, new_ptr: *mut u8, old_size: usize, new_size: usize) {
        METRICS.record_global_deallocation(old_size);
        METRICS.record_global_allocation(new_size);

        IN_TRACKING.with(|active| {
            if active.get() {
                return;
            }
            active.set(true);

            let old_meta = METRICS.take_allocation_metadata(old_ptr as usize);
            if let Some(old_meta) = old_meta {
                METRICS.record_sampled_deallocation(old_meta.size, old_meta.stack_id);
                let stack_id = old_meta.stack_id;
                let meta = AllocationMeta { size: new_size, stack_id };
                let stored = METRICS.store_allocation_metadata(new_ptr as usize, meta);
                if stored {
                    METRICS.record_sampled_allocation(new_size, stack_id);
                }
            } else if should_sample_allocation() {
                let stack_id = capture_stack_id();
                let meta = AllocationMeta { size: new_size, stack_id };
                let stored = METRICS.store_allocation_metadata(new_ptr as usize, meta);
                if stored {
                    METRICS.record_sampled_allocation(new_size, stack_id);
                }
            }

            active.set(false);
        });
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for TrackingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.inner.alloc(layout);
        if !ptr.is_null() {
            self.track_alloc(ptr, layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner.dealloc(ptr, layout);
        self.track_dealloc(ptr, layout.size());
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = self.inner.alloc_zeroed(layout);
        if !ptr.is_null() {
            self.track_alloc(ptr, layout.size());
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let new_ptr = self.inner.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            self.track_realloc(ptr, new_ptr, layout.size(), new_size);
        }
        new_ptr
    }
}

// Pre-built allocator types for common use cases
/// Tracking allocator using the system allocator
pub type TrackingSystem = TrackingAllocator<System>;

#[cfg(feature = "jemalloc")]
/// Tracking allocator using jemalloc (when jemalloc feature is enabled)
pub type TrackingJemalloc = TrackingAllocator<jemallocator::Jemalloc>;

#[cfg(feature = "mimalloc")]
/// Tracking allocator using mimalloc (when mimalloc feature is enabled)
pub type TrackingMimalloc = TrackingAllocator<mimalloc::MiMalloc>;

// Default global allocator using system allocator
#[global_allocator]
static ALLOCATOR: TrackingSystem = TrackingSystem::new(System);