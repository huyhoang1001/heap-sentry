use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};

use crate::metrics::METRICS;

#[cfg(feature = "backtrace")]
use backtrace::Backtrace;

pub static ENABLE_BACKTRACE: AtomicUsize = AtomicUsize::new(0);

/// Global allocator that wraps the system allocator and tracks allocations
pub struct TrackingAllocator {
    system: System,
}

#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator {
    system: System,
};

impl TrackingAllocator {
    /// Track an allocation
    fn track_alloc(&self, size: usize) {
        METRICS.total_allocated.fetch_add(size, Ordering::Relaxed);
        METRICS.allocation_count.fetch_add(1, Ordering::Relaxed);
        METRICS.current_usage.fetch_add(size, Ordering::Relaxed);

        #[cfg(feature = "backtrace")]
        if ENABLE_BACKTRACE.load(Ordering::Relaxed) == 1 {
            let bt = Backtrace::new();
            let key = format!("{:?}", bt);
            if let Ok(mut map) = METRICS.callsites.lock() {
                let entry = map.entry(key).or_insert_with(|| crate::metrics::AllocationStats { allocated: 0, count: 0 });
                entry.allocated += size;
                entry.count += 1;
            } // If mutex is poisoned, we skip backtrace tracking but continue
        }
    }

    /// Track a deallocation
    fn track_dealloc(&self, size: usize) {
        METRICS.total_freed.fetch_add(size, Ordering::Relaxed);
        METRICS.deallocation_count.fetch_add(1, Ordering::Relaxed);
        METRICS.current_usage.fetch_sub(size, Ordering::Relaxed);
    }
}

unsafe impl GlobalAlloc for TrackingAllocator {
    unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
        let ptr = self.system.alloc(layout);
        if !ptr.is_null() {
            self.track_alloc(layout.size());
        }
        ptr
    }

    unsafe fn dealloc(&self, ptr: *mut u8, layout: Layout) {
        self.system.dealloc(ptr, layout);
        self.track_dealloc(layout.size());
    }

    unsafe fn alloc_zeroed(&self, layout: Layout) -> *mut u8 {
        let ptr = self.system.alloc_zeroed(layout);
        if !ptr.is_null() {
            self.track_alloc(layout.size());
        }
        ptr
    }

    unsafe fn realloc(&self, ptr: *mut u8, layout: Layout, new_size: usize) -> *mut u8 {
        let old_size = layout.size();
        let new_ptr = self.system.realloc(ptr, layout, new_size);
        if !new_ptr.is_null() {
            self.track_dealloc(old_size);
            self.track_alloc(new_size);
        }
        new_ptr
    }
}