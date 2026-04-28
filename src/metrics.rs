use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};
use std::collections::HashMap;

use lazy_static::lazy_static;

#[cfg(feature = "tracing")]
use tracing::warn;

/// Custom error type for metrics operations
#[derive(Debug, Clone)]
pub enum MetricsError {
    MutexPoisoned,
    InvalidData,
}

/// Result type for metrics operations
pub type MetricsResult<T> = Result<T, MetricsError>;

/// Safe mutex wrapper that handles poisoning gracefully
#[derive(Debug)]
pub struct SafeMutex<T> {
    mutex: Mutex<T>,
}

impl<T> SafeMutex<T> {
    pub fn new(data: T) -> Self {
        Self {
            mutex: Mutex::new(data),
        }
    }

    /// Lock with poisoning recovery
    pub fn lock(&self) -> MetricsResult<std::sync::MutexGuard<T>> {
        match self.mutex.lock() {
            Ok(guard) => Ok(guard),
            Err(PoisonError { .. }) => {
                // Log the poisoning but continue with recovered mutex
                #[cfg(feature = "tracing")]
                warn!("Metrics mutex was poisoned, recovering...");
                eprintln!("[WARN] Metrics mutex was poisoned, recovering...");
                Err(MetricsError::MutexPoisoned)
            }
        }
    }

    /// Try to lock, returning None if would block or poisoned
    pub fn try_lock(&self) -> Option<std::sync::MutexGuard<T>> {
        match self.mutex.try_lock() {
            Ok(guard) => Some(guard),
            Err(_) => None, // Either poisoned or would block
        }
    }
}

/// Statistics for allocation call sites
#[derive(Debug)]
pub struct AllocationStats {
    pub allocated: usize, // Total bytes allocated from this call site
    pub count: usize,     // Number of allocations from this call site
}

/// Global metrics storage
#[derive(Debug)]
pub struct Metrics {
    pub total_allocated: AtomicUsize,
    pub total_freed: AtomicUsize,
    pub current_usage: AtomicUsize,
    pub allocation_count: AtomicUsize,
    pub deallocation_count: AtomicUsize,
    pub callsites: SafeMutex<HashMap<String, AllocationStats>>, // Only used with backtrace feature
}

lazy_static! {
    pub static ref METRICS: Metrics = Metrics {
        total_allocated: AtomicUsize::new(0),
        total_freed: AtomicUsize::new(0),
        current_usage: AtomicUsize::new(0),
        allocation_count: AtomicUsize::new(0),
        deallocation_count: AtomicUsize::new(0),
        callsites: SafeMutex::new(HashMap::new()),
    };
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

/// Per-thread memory statistics
#[derive(Debug)]
pub struct ThreadMemoryStats {
    /// Thread ID
    pub thread_id: String,
    /// Thread name
    pub thread_name: String,
    /// Bytes allocated by this thread
    pub allocated: usize,
    /// Bytes freed by this thread
    pub freed: usize,
    /// Current memory usage by this thread
    pub current_usage: usize,
    /// Allocation count for this thread
    pub allocation_count: usize,
    /// Deallocation count for this thread
    pub deallocation_count: usize,
    /// Thread uptime
    pub uptime_seconds: u64,
}

/// Get memory statistics for all threads
/// Note: Currently returns global stats as per-thread tracking is complex
pub fn thread_stats() -> Vec<ThreadMemoryStats> {
    // For now, return global stats as a single "main" thread
    // Per-thread tracking requires more complex implementation
    vec![ThreadMemoryStats {
        thread_id: "main".to_string(),
        thread_name: "main".to_string(),
        allocated: METRICS.total_allocated.load(Ordering::Relaxed),
        freed: METRICS.total_freed.load(Ordering::Relaxed),
        current_usage: METRICS.current_usage.load(Ordering::Relaxed),
        allocation_count: METRICS.allocation_count.load(Ordering::Relaxed),
        deallocation_count: METRICS.deallocation_count.load(Ordering::Relaxed),
        uptime_seconds: 0, // Not tracked
    }]
}

/// Get current thread's memory statistics
/// Note: Returns global stats as per-thread tracking is not yet implemented
pub fn current_thread_stats() -> ThreadMemoryStats {
    ThreadMemoryStats {
        thread_id: "current".to_string(),
        thread_name: "current".to_string(),
        allocated: METRICS.total_allocated.load(Ordering::Relaxed),
        freed: METRICS.total_freed.load(Ordering::Relaxed),
        current_usage: METRICS.current_usage.load(Ordering::Relaxed),
        allocation_count: METRICS.allocation_count.load(Ordering::Relaxed),
        deallocation_count: METRICS.deallocation_count.load(Ordering::Relaxed),
        uptime_seconds: 0,
    }
}