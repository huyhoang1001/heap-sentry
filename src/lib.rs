//! # Heap Sentry
//!
//! A lightweight Rust library for detecting memory leaks and unbounded memory growth.
//!
//! ## Features
//!
//! - **Memory Leak Detection**: Identifies when allocated memory exceeds freed memory by a threshold
//! - **Growth Monitoring**: Detects sustained memory growth rates above configurable limits
//! - **Call Site Tracking**: Optional backtrace collection to identify allocation hotspots
//! - **Low Overhead**: Uses atomic operations and background sampling for minimal performance impact
//!
//! ## Usage
//!
//! ```rust
//! use heap_sentry::{init_tracker, TrackerConfig};
//!
//! fn main() {
//!     init_tracker(TrackerConfig::default());
//!     // Your application code here
//! }
//! ```

use std::alloc::{GlobalAlloc, Layout, System};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Mutex;
use std::collections::HashMap;
use std::time::{Duration, Instant};
use std::thread;

#[cfg(feature = "backtrace")]
use backtrace::Backtrace;

use lazy_static::lazy_static;

/// Configuration for the heap sentry tracker
#[derive(Debug)]
pub struct TrackerConfig {
    /// Sampling interval in milliseconds
    pub sampling_interval_ms: u64,
    /// Growth threshold in bytes per second
    pub growth_threshold_bytes_per_sec: usize,
    /// Leak threshold in bytes
    pub leak_threshold_bytes: usize,
    /// Enable backtrace collection for call sites
    pub enable_backtrace: bool,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            sampling_interval_ms: 1000,
            growth_threshold_bytes_per_sec: 1024 * 1024, // 1MB/s
            leak_threshold_bytes: 10 * 1024 * 1024, // 10MB
            enable_backtrace: false,
        }
    }
}

struct AllocationStats {
    allocated: usize, // Total bytes allocated from this call site
    count: usize,     // Number of allocations from this call site
}

struct Metrics {
    total_allocated: AtomicUsize,
    total_freed: AtomicUsize,
    current_usage: AtomicUsize,
    allocation_count: AtomicUsize,
    deallocation_count: AtomicUsize,
    callsites: Mutex<HashMap<String, AllocationStats>>, // Only used with backtrace feature
}

lazy_static! {
    static ref METRICS: Metrics = Metrics {
        total_allocated: AtomicUsize::new(0),
        total_freed: AtomicUsize::new(0),
        current_usage: AtomicUsize::new(0),
        allocation_count: AtomicUsize::new(0),
        deallocation_count: AtomicUsize::new(0),
        callsites: Mutex::new(HashMap::new()),
    };
}

/// Global allocator that wraps the system allocator and tracks allocations
struct TrackingAllocator {
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
            let mut map = METRICS.callsites.lock().unwrap();
            let entry = map.entry(key).or_insert(AllocationStats { allocated: 0, count: 0 });
            entry.allocated += size;
            entry.count += 1;
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

static ENABLE_BACKTRACE: AtomicUsize = AtomicUsize::new(0);

/// Initialize the heap sentry tracker with the given configuration.
/// This starts a background thread that monitors memory usage.
pub fn init_tracker(config: TrackerConfig) {
    ENABLE_BACKTRACE.store(config.enable_backtrace as usize, Ordering::Relaxed);
    // Start sampling thread
    thread::spawn(move || {
        let mut samples: Vec<Sample> = Vec::new();
        let interval = Duration::from_millis(config.sampling_interval_ms);
        loop {
            thread::sleep(interval);
            let usage = METRICS.current_usage.load(Ordering::Relaxed);
            let sample = Sample {
                timestamp: Instant::now(),
                memory_usage: usage,
            };
            samples.push(sample);
            // Keep only last 100 samples
            if samples.len() > 100 {
                samples.remove(0);
            }
            // Analyze
            analyze_and_report(&samples, &config);
        }
    });
}

#[derive(Clone)]
struct Sample {
    timestamp: Instant,
    memory_usage: usize,
}

/// Analyze samples and report issues
fn analyze_and_report(samples: &[Sample], config: &TrackerConfig) {
    if samples.len() < 2 {
        return;
    }
    let total_allocated = METRICS.total_allocated.load(Ordering::Relaxed);
    let total_freed = METRICS.total_freed.load(Ordering::Relaxed);
    let leak_size = total_allocated.saturating_sub(total_freed);
    if leak_size > config.leak_threshold_bytes {
        // Check if not decreasing over last 10 samples
        let start = samples.len().saturating_sub(10);
        let recent = &samples[start..];
        let decreasing = recent.windows(2).all(|w| w[1].memory_usage <= w[0].memory_usage);
        if !decreasing {
            eprintln!("[WARN] Potential memory leak detected: {} bytes not freed", leak_size);
        }
    }
    // Growth detection
    let first = &samples[0];
    let last = &samples[samples.len() - 1];
    let time_diff = last.timestamp.duration_since(first.timestamp).as_secs_f64();
    if time_diff > 0.0 {
        let growth = last.memory_usage as f64 - first.memory_usage as f64;
        let rate = growth / time_diff;
        if rate > config.growth_threshold_bytes_per_sec as f64 {
            eprintln!("[WARN] Unbounded memory growth detected: {:.2} bytes/sec", rate);
        }
    }
    // Hotspots
    #[cfg(feature = "backtrace")]
    if config.enable_backtrace {
        let map = METRICS.callsites.lock().unwrap();
        let mut hotspots: Vec<_> = map.iter().collect();
        hotspots.sort_by(|a, b| b.1.allocated.cmp(&a.1.allocated));
        if !hotspots.is_empty() {
            eprintln!("Top allocation sources:");
            for (callsite, stats) in hotspots.iter().take(5) {
                eprintln!("  - {}: {} bytes", callsite, stats.allocated);
            }
        }
    }
}

/// Take a snapshot of current memory statistics
pub fn snapshot() -> MemoryStats {
    MemoryStats {
        total_allocated: METRICS.total_allocated.load(Ordering::Relaxed),
        total_freed: METRICS.total_freed.load(Ordering::Relaxed),
        current_usage: METRICS.current_usage.load(Ordering::Relaxed),
        allocation_count: METRICS.allocation_count.load(Ordering::Relaxed),
        deallocation_count: METRICS.deallocation_count.load(Ordering::Relaxed),
    }
}

/// Memory statistics snapshot
#[derive(Debug)]
pub struct MemoryStats {
    /// Total bytes allocated since tracking started
    pub total_allocated: usize,
    /// Total bytes freed since tracking started
    pub total_freed: usize,
    /// Current memory usage (allocated - freed)
    pub current_usage: usize,
    /// Total number of allocation operations
    pub allocation_count: usize,
    /// Total number of deallocation operations
    pub deallocation_count: usize,
}