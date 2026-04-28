use std::alloc::{GlobalAlloc, Layout, System};
#[cfg(feature = "backtrace")]
use std::cell::Cell;
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::metrics::METRICS;

#[cfg(feature = "backtrace")]
use backtrace::Backtrace;

#[cfg(feature = "backtrace")]
thread_local! {
    static BACKTRACE_CAPTURE_ACTIVE: Cell<bool> = Cell::new(false);
}

pub static ENABLE_BACKTRACE: AtomicUsize = AtomicUsize::new(0);

/// Generic allocator wrapper that tracks allocations for any underlying allocator
pub struct TrackingAllocator<A: GlobalAlloc> {
    inner: A,
}

impl<A: GlobalAlloc> TrackingAllocator<A> {
    /// Create a new tracking allocator wrapping the given allocator
    pub const fn new(inner: A) -> Self {
        Self { inner }
    }

    /// Track an allocation
    fn track_alloc(&self, size: usize) {
        METRICS.total_allocated.fetch_add(size, Ordering::Relaxed);
        METRICS.allocation_count.fetch_add(1, Ordering::Relaxed);
        METRICS.current_usage.fetch_add(size, Ordering::Relaxed);

        #[cfg(feature = "backtrace")]
        if ENABLE_BACKTRACE.load(Ordering::Relaxed) == 1 {
            BACKTRACE_CAPTURE_ACTIVE.with(|active| {
                if active.get() {
                    return;
                }
                active.set(true);

                let bt = Backtrace::new();
                let key = format!("{:?}", bt);
                if let Ok(mut map) = METRICS.callsites.lock() {
                    let entry = map.entry(key).or_insert_with(|| crate::metrics::AllocationStats { allocated: 0, count: 0 });
                    entry.allocated += size;
                    entry.count += 1;
                }

                active.set(false);
            });
        }
    }

    /// Track a deallocation
    fn track_dealloc(&self, size: usize) {
        METRICS.total_freed.fetch_add(size, Ordering::Relaxed);
        METRICS.deallocation_count.fetch_add(1, Ordering::Relaxed);
        METRICS.current_usage.fetch_sub(size, Ordering::Relaxed);
    }
}

unsafe impl<A: GlobalAlloc> GlobalAlloc for TrackingAllocator<A> {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.inner.alloc(layout);
        if !ptr.is_null() {
            self.track_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.inner.dealloc(ptr, layout);
        self.track_dealloc(layout.size());
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = self.inner.alloc_zeroed(layout);
        if !ptr.is_null() {
            self.track_alloc(layout.size());
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_size = layout.size();
        let new_ptr = self.inner.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            self.track_dealloc(old_size);
            self.track_alloc(new_size);
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