# How Heap Sentry Works

Heap Sentry is a lightweight Rust library that detects memory leaks and monitors unbounded memory growth in real-time. This document explains the internal workings, algorithms, and design decisions.

## Architecture Overview

```
┌─────────────────┐
│   User Application │
└─────────┬───────┘
          │
          v
┌─────────────────┐    ┌─────────────────┐    ┌─────────────────┐
│ Global Allocator │ -> │   Metrics       │ -> │   Analyzer      │
│   Wrapper        │    │   Collector     │    │   Engine        │
└─────────────────┘    └─────────────────┘    └─────────────────┘
          │                       │                       │
          └───────────────────────┼───────────────────────┘
                                  v
                         ┌─────────────────┐
                         │   Reporter      │
                         │   (stderr)      │
                         └─────────────────┘
```

## 1. Global Allocator Wrapper

### How It Works

Heap Sentry installs itself as Rust's global allocator using the `#[global_allocator]` attribute. Every memory allocation and deallocation in the program goes through this wrapper.

```rust
#[global_allocator]
static ALLOCATOR: TrackingAllocator = TrackingAllocator {
    system: System, // Wraps the system allocator
};
```

### Tracking Mechanism

For each allocation/deallocation, the wrapper:

1. **Delegates to system allocator**: Calls the underlying `System` allocator
2. **Updates metrics atomically**: Uses `AtomicUsize` for thread-safe counters
3. **Captures backtraces (optional)**: When enabled, records call sites

```rust
unsafe fn alloc(&self, layout: Layout) -> *mut u8 {
    let ptr = self.system.alloc(layout);
    if !ptr.is_null() {
        self.track_alloc(layout.size());
    }
    ptr
}
```

### Metrics Collected

- `total_allocated`: Total bytes allocated since startup
- `total_freed`: Total bytes freed since startup
- `current_usage`: Current memory usage (allocated - freed)
- `allocation_count`: Number of allocation operations
- `deallocation_count`: Number of deallocation operations

## 2. Metrics Collector

### Thread-Safe Storage

All metrics use `AtomicUsize` for lock-free, thread-safe updates:

```rust
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
```

### Performance Design

- **Atomic operations**: No locks on the hot path
- **Lazy initialization**: Metrics created only when first accessed
- **Minimal overhead**: Counters only, no complex logic in alloc/dealloc

## 3. Sampling Engine

### Background Monitoring

When `init_tracker()` is called, a background thread starts:

```rust
thread::spawn(move || {
    let mut samples: Vec<Sample> = Vec::new();
    loop {
        thread::sleep(interval);
        let usage = METRICS.current_usage.load(Ordering::Relaxed);
        samples.push(Sample { timestamp: Instant::now(), memory_usage: usage });
        // Keep last 100 samples
        if samples.len() > 100 { samples.remove(0); }
        analyze_and_report(&samples, &config);
    }
});
```

### Sample Structure

```rust
struct Sample {
    timestamp: Instant,    // When the sample was taken
    memory_usage: usize,   // Current memory usage in bytes
}
```

### Why Sampling?

- **Performance**: Continuous monitoring would be too expensive
- **Trend analysis**: Need time-series data to detect patterns
- **Configurable**: Users can adjust sampling frequency

## 4. Analyzer Engine

### Memory Leak Detection

**Algorithm**: Compares total allocated vs total freed over time.

```rust
let leak_size = total_allocated - total_freed;
if leak_size > config.leak_threshold_bytes {
    // Check if memory is NOT decreasing over recent samples
    let recent = &samples[samples.len().saturating_sub(10)..];
    let decreasing = recent.windows(2).all(|w| w[1].memory_usage <= w[0].memory_usage);
    if !decreasing {
        report_leak(leak_size);
    }
}
```

**Logic**: A leak is suspected when:
1. Total unfreed memory exceeds threshold
2. Memory usage is not decreasing over recent samples

### Unbounded Growth Detection

**Algorithm**: Calculates memory growth rate over time.

```rust
let first = &samples[0];
let last = &samples[samples.len() - 1];
let time_diff = last.timestamp.duration_since(first.timestamp).as_secs_f64();
if time_diff > 0.0 {
    let growth = last.memory_usage as f64 - first.memory_usage as f64;
    let rate = growth / time_diff; // bytes per second
    if rate > config.growth_threshold_bytes_per_sec as f64 {
        report_growth(rate);
    }
}
```

**Logic**: Growth is detected when the average growth rate exceeds the threshold.

### Call Site Analysis (Optional)

When backtrace feature is enabled:

1. **Capture backtrace** on each allocation
2. **Hash and store** compact representation
3. **Aggregate by call site** to find hotspots

```rust
#[cfg(feature = "backtrace")]
if ENABLE_BACKTRACE.load(Ordering::Relaxed) == 1 {
    let bt = Backtrace::new();
    let key = format!("{:?}", bt); // Hash the backtrace
    // Update call site statistics
}
```

## 5. Reporter

### Output Format

Warnings are printed to stderr:

```
[WARN] Potential memory leak detected: 1048576 bytes not freed
[WARN] Unbounded memory growth detected: 524288.00 bytes/sec
```

### Design Decisions

- **Stderr**: Doesn't interfere with application output
- **Non-blocking**: Uses simple `eprintln!` for reliability
- **Actionable**: Includes specific numbers for debugging

## 6. Configuration Options

### TrackerConfig

```rust
struct TrackerConfig {
    sampling_interval_ms: u64,                    // How often to sample (default: 1000ms)
    growth_threshold_bytes_per_sec: usize,        // Growth rate limit (default: 1MB/s)
    leak_threshold_bytes: usize,                  // Leak size limit (default: 10MB)
    enable_backtrace: bool,                       // Enable call site tracking (default: false)
}
```

### Performance Trade-offs

- **Higher sampling rate**: More accurate detection, higher overhead
- **Lower thresholds**: More sensitive detection, more false positives
- **Backtrace enabled**: Detailed diagnostics, significant performance cost

## 7. Performance Characteristics

### Overhead Analysis

- **Allocation path**: ~10-20 atomic operations per allocation
- **Background thread**: Minimal CPU usage, sleeps most of the time
- **Memory usage**: O(1) for basic mode, O(number of call sites) for backtrace mode

### Thread Safety

- **Lock-free**: Hot path uses only atomic operations
- **Mutex-protected**: Call site map uses Mutex (only when backtrace enabled)
- **No blocking**: Background thread doesn't block application threads

### Memory Overhead

- **Basic mode**: ~100 bytes for static metrics + sample buffer
- **Backtrace mode**: Additional memory for call site storage
- **Sample buffer**: Fixed size (100 samples × 24 bytes ≈ 2.4KB)

## 8. Detection Accuracy

### Leak Detection

- **Conservative approach**: Only reports when clearly suspicious
- **Time-based validation**: Requires sustained non-decreasing usage
- **Threshold-based**: Configurable sensitivity

### Growth Detection

- **Trend analysis**: Uses all available samples for rate calculation
- **Linear approximation**: Assumes constant growth rate
- **Configurable sensitivity**: Users can adjust thresholds

### Limitations

- **No false positive guarantees**: May report issues that aren't real problems
- **Sampling-based**: May miss short-lived growth spikes
- **Allocation-based**: Only tracks heap allocations, not stack or other memory

## 9. Integration Points

### Initialization

```rust
fn main() {
    // Must be called before any allocations
    init_tracker(TrackerConfig::default());
    // Application code...
}
```

### Global Allocator

The `#[global_allocator]` attribute ensures all allocations go through Heap Sentry. This happens at compile time and cannot be changed at runtime.

### Threading Model

- **Single background thread**: Handles sampling and analysis
- **Non-blocking**: Application threads never wait for monitoring
- **Graceful shutdown**: Background thread runs until program exit

## 10. Future Enhancements

### Potential Improvements

- **Prometheus metrics export**: Integration with monitoring systems
- **File-based reporting**: Write reports to disk
- **Custom allocators**: Support for jemalloc, mimalloc, etc.
- **Memory profiling**: Detailed allocation patterns
- **Web dashboard**: Real-time visualization

### Research Areas

- **Machine learning**: Pattern recognition for complex leaks
- **Predictive analysis**: Forecast memory issues before they occur
- **Cross-process monitoring**: Track memory across process boundaries

## Conclusion

Heap Sentry provides a balance between detection capability and performance overhead. By using sampling, atomic operations, and conservative algorithms, it can detect real memory issues in production systems with minimal impact on application performance.

The design prioritizes **actionable insights** over **perfect accuracy**, making it practical for real-world usage where some false positives are acceptable to catch genuine issues.