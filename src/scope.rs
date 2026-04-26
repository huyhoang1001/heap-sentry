use crate::metrics::{snapshot, MemoryStats};

/// Scoped memory tracking guard
pub struct MemoryScope {
    name: String,
    start_stats: MemoryStats,
}

impl MemoryScope {
    /// Create a new memory scope with the given name
    pub fn new(name: impl Into<String>) -> Self {
        let name = name.into();
        let start_stats = snapshot();
        Self { name, start_stats }
    }

    /// Get memory statistics for this scope
    pub fn stats(&self) -> ScopedStats {
        let current = snapshot();
        ScopedStats {
            name: self.name.clone(),
            allocated: current.total_allocated - self.start_stats.total_allocated,
            freed: current.total_freed - self.start_stats.total_freed,
            peak_usage: 0, // TODO: Track peak usage
            allocation_count: current.allocation_count - self.start_stats.allocation_count,
            deallocation_count: current.deallocation_count - self.start_stats.deallocation_count,
        }
    }
}

impl Drop for MemoryScope {
    fn drop(&mut self) {
        let stats = self.stats();
        if stats.allocated > 1024 * 1024 { // Only report if > 1MB allocated
            eprintln!("[INFO] Memory scope '{}' completed: {} bytes allocated, {} bytes net",
                     stats.name, stats.allocated, stats.allocated as i64 - stats.freed as i64);
        }
    }
}

/// Statistics for a memory scope
#[derive(Debug)]
pub struct ScopedStats {
    /// Scope name
    pub name: String,
    /// Total bytes allocated in this scope
    pub allocated: usize,
    /// Total bytes freed in this scope
    pub freed: usize,
    /// Peak memory usage in this scope
    pub peak_usage: usize,
    /// Number of allocations in this scope
    pub allocation_count: usize,
    /// Number of deallocations in this scope
    pub deallocation_count: usize,
}

/// Macro for scoped memory tracking
#[macro_export]
macro_rules! track_scope {
    ($name:expr, $code:block) => {{
        let _scope = $crate::MemoryScope::new($name);
        $code
    }};
}