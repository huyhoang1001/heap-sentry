use std::collections::HashMap;
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};
use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::{Mutex, PoisonError};

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
            Err(_) => None,
        }
    }
}

/// Active allocation metadata for pointer tracking
#[derive(Debug)]
pub struct AllocationMeta {
    pub size: usize,
    pub stack_id: u64,
}

/// Aggregated allocation statistics for a stack trace
#[derive(Debug, Clone)]
pub struct AllocationStat {
    pub total_bytes: usize,
    pub live_bytes: usize,
    pub alloc_count: usize,
    pub dealloc_count: usize,
}

impl AllocationStat {
    pub fn new() -> Self {
        Self {
            total_bytes: 0,
            live_bytes: 0,
            alloc_count: 0,
            dealloc_count: 0,
        }
    }
}

/// Global metrics storage
#[derive(Debug)]
pub struct Metrics {
    pub total_allocated: AtomicUsize,
    pub total_freed: AtomicUsize,
    pub current_usage: AtomicUsize,
    pub allocation_count: AtomicUsize,
    pub deallocation_count: AtomicUsize,
    pub active_allocations: SafeMutex<HashMap<usize, AllocationMeta>>,
    pub stack_traces: SafeMutex<HashMap<u64, String>>,
    pub allocation_stats: SafeMutex<HashMap<u64, AllocationStat>>,
}

lazy_static! {
    pub static ref METRICS: Metrics = Metrics {
        total_allocated: AtomicUsize::new(0),
        total_freed: AtomicUsize::new(0),
        current_usage: AtomicUsize::new(0),
        allocation_count: AtomicUsize::new(0),
        deallocation_count: AtomicUsize::new(0),
        active_allocations: SafeMutex::new(HashMap::new()),
        stack_traces: SafeMutex::new(HashMap::new()),
        allocation_stats: SafeMutex::new(HashMap::new()),
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

impl Metrics {
    fn hash_backtrace(backtrace: &str) -> u64 {
        let mut hasher = DefaultHasher::new();
        backtrace.hash(&mut hasher);
        hasher.finish()
    }

    pub fn record_stack_trace(&self, backtrace: String) -> u64 {
        let stack_id = Self::hash_backtrace(&backtrace);
        if let Ok(mut traces) = self.stack_traces.lock() {
            traces.entry(stack_id).or_insert(backtrace);
        }
        stack_id
    }

    pub fn record_allocation(&self, size: usize, stack_id: u64) {
        self.total_allocated.fetch_add(size, Ordering::Relaxed);
        self.allocation_count.fetch_add(1, Ordering::Relaxed);
        self.current_usage.fetch_add(size, Ordering::Relaxed);

        if stack_id != 0 {
            if let Ok(mut stats) = self.allocation_stats.lock() {
                let entry = stats.entry(stack_id).or_insert_with(AllocationStat::new);
                entry.total_bytes += size;
                entry.live_bytes += size;
                entry.alloc_count += 1;
            }
        }
    }

    pub fn record_deallocation(&self, size: usize, stack_id: u64) {
        self.total_freed.fetch_add(size, Ordering::Relaxed);
        self.deallocation_count.fetch_add(1, Ordering::Relaxed);
        self.current_usage.fetch_sub(size, Ordering::Relaxed);

        if stack_id != 0 {
            if let Ok(mut stats) = self.allocation_stats.lock() {
                if let Some(entry) = stats.get_mut(&stack_id) {
                    entry.live_bytes = entry.live_bytes.saturating_sub(size);
                    entry.dealloc_count += 1;
                }
            }
        }
    }

    pub fn store_allocation_metadata(&self, ptr: usize, meta: AllocationMeta) {
        if let Ok(mut allocations) = self.active_allocations.lock() {
            allocations.insert(ptr, meta);
        }
    }

    pub fn take_allocation_metadata(&self, ptr: usize) -> Option<AllocationMeta> {
        if let Ok(mut allocations) = self.active_allocations.lock() {
            allocations.remove(&ptr)
        } else {
            None
        }
    }

    pub fn top_allocation_stats(&self, limit: usize) -> Vec<(u64, AllocationStat, Option<String>)> {
        let mut items = Vec::new();
        if let Ok(stats) = self.allocation_stats.lock() {
            for (stack_id, stat) in stats.iter() {
                let trace = self.stack_traces.lock().ok().and_then(|traces| traces.get(stack_id).cloned());
                items.push((*stack_id, stat.clone(), trace));
            }
        }
        items.sort_by(|a, b| b.1.live_bytes.cmp(&a.1.live_bytes));
        items.truncate(limit);
        items
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