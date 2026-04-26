# Heap Sentry

Heap Sentry is a lightweight Rust library for tracking heap allocations, detecting memory leaks, and identifying unbounded memory growth. It provides real-time metrics and actionable insights with low overhead in production and deeper diagnostics in debug mode.

## Features

- **Memory Leak Detection**: Identifies when allocated memory exceeds freed memory by a threshold
- **Growth Monitoring**: Detects sustained memory growth rates above configurable limits
- **Scoped Tracking**: Fine-grained memory monitoring for specific code blocks and operations
- **Multi-threaded Support**: Global memory tracking across complex applications
- **Call Site Tracking**: Optional backtrace collection to identify allocation hotspots (with `backtrace` feature)
- **Low Overhead**: Uses atomic operations and background sampling for minimal performance impact

## Documentation

- **[How It Works](docs/how_it_works.md)** - Detailed explanation of the architecture, algorithms, and internal workings
- **[Enhancement Proposals](docs/enhancement_proposals.md)** - Future improvements and planned features

## Quick Start

Add to your `Cargo.toml`:

```toml
[dependencies]
heap-sentry = "0.1"
```

Basic usage:

```rust
use heap_sentry::{init_tracker, TrackerConfig};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize with default configuration
    init_tracker(TrackerConfig::default())?;

    // Your application code here
    loop {
        let data = vec![0u8; 1024 * 1024]; // simulate allocation
        std::thread::sleep(std::time::Duration::from_millis(100));
    }

    Ok(())
}
```

The library will automatically detect memory leaks and unbounded growth, reporting warnings to stderr.

## Scoped Tracking

Track memory usage for specific code blocks to identify problematic operations:

```rust
use heap_sentry::{MemoryScope, track_scope};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    init_tracker(TrackerConfig::default())?;

    // Manual scope tracking
    {
        let _scope = MemoryScope::new("data_processing");
        let data = vec![0u8; 1024 * 1024];
        // Memory allocated here is tracked under "data_processing"
        process_data(data);
    } // Scope ends, memory usage is reported

    // Macro-based scope tracking (more convenient)
    track_scope!("file_operations", {
        let file_data = vec![0u8; 512 * 1024];
        // Memory allocated here is tracked under "file_operations"
        write_to_file(file_data);
    });

    Ok(())
}
```

### Scope Output

```
[INFO] Memory scope 'data_processing' completed: 1048576 bytes allocated, 0 bytes net
[INFO] Memory scope 'file_operations' completed: 524288 bytes allocated, 0 bytes net
```

## Configuration

### Built-in Configurations

```rust
use heap_sentry::TrackerConfig;

// Default configuration (balanced for most applications)
let config = TrackerConfig::default();

// Debug mode: More sensitive detection, enables backtraces
let config = TrackerConfig::debug();

// Production mode: Conservative settings for minimal overhead
let config = TrackerConfig::production();
```

### Custom Configuration

```rust
use heap_sentry::config::OutputFormat;

let config = TrackerConfig {
    sampling_interval_ms: 1000,              // Sample every second
    growth_threshold_bytes_per_sec: 1024 * 1024, // 1MB/s growth threshold
    leak_threshold_bytes: 10 * 1024 * 1024,     // 10MB leak threshold
    enable_backtrace: false,                     // Disable call site tracking
    output_format: OutputFormat::Stderr,         // Output format
}.validate()?; // Always validate custom configs
```

### Output Formats

Heap Sentry supports different output formats for integration with monitoring systems:

```rust
// Standard error output (default)
output_format: OutputFormat::Stderr

// Structured JSON output
output_format: OutputFormat::JsonStderr

// File output (planned for future release)
output_format: OutputFormat::File("memory.log".to_string())
```

### Configuration Options

- `sampling_interval_ms`: How often to check memory usage (default: 1000ms)
- `growth_threshold_bytes_per_sec`: Maximum allowed memory growth rate (default: 1MB/s)
- `leak_threshold_bytes`: Minimum leak size to report (default: 10MB)
- `enable_backtrace`: Enable call site tracking (requires `backtrace` feature, default: false)

## Multi-Threaded Applications

Heap Sentry provides robust support for multi-threaded applications with global memory tracking:

### Global Memory Statistics

```rust
use heap_sentry::{thread_stats, current_thread_stats, snapshot};

// Get global statistics (currently returns as single entry)
let stats = thread_stats();
for thread in stats {
    println!("Global memory: {} MB used, {} allocations",
             thread.current_usage / (1024 * 1024),
             thread.allocation_count);
}

// Get current thread statistics (returns global stats)
let my_stats = current_thread_stats();

// Take a snapshot of current memory state
let snapshot = snapshot();
println!("Current usage: {} bytes", snapshot.current_usage);
```

### Detection Capabilities

The library monitors your entire multi-threaded application for:

- **Global memory leak detection** across all threads
- **Global growth rate monitoring** for unbounded memory usage
- **Scoped tracking** to monitor specific operations within threads
- **Thread-safe metrics** using atomic operations

### Example Output

```
[WARN] Potential memory leak detected: 5242880 bytes not freed
  Use scoped tracking or enable the backtrace feature to identify which parts of your application are leaking.

[WARN] Unbounded memory growth detected: 1048576.00 bytes/sec
  Use scoped tracking or enable the backtrace feature to identify which code paths are growing.
```

### Best Practices for Multi-threaded Apps

1. **Use scoped tracking** within threads to identify problematic operations
2. **Initialize early** in `main()` before spawning threads
3. **Monitor global statistics** for overall application health
4. **Use debug configuration** during development for more sensitive detection

## Examples

Run the included examples to see Heap Sentry in action:

```bash
# Basic leak detection - demonstrates memory leak identification
cargo run --example leak_example

# Growth rate detection - shows unbounded memory growth detection
cargo run --example growth_example

# Scoped memory tracking - demonstrates fine-grained monitoring
cargo run --example scoped_example

# Multi-threaded memory tracking - shows global monitoring across threads
cargo run --example threaded_example

# JSON structured output - demonstrates machine-readable output
cargo run --example json_output_example

# Call site tracking (requires backtrace feature)
cargo run --example backtrace_example --features backtrace
```

### Example Output

```
Starting leak detection example...
[WARN] Potential memory leak detected: 1101597 bytes not freed
  Use scoped tracking to identify which parts of your application are leaking.

Starting scoped tracking example...
[INFO] Memory scope 'manual_scope' completed: 2097152 bytes allocated, 0 bytes net
[INFO] Memory scope 'macro_scope' completed: 2621440 bytes allocated, 0 bytes net
```

## Performance

Heap Sentry is designed for minimal overhead:

- **Allocation overhead**: ~10-20 atomic operations per allocation/deallocation
- **Memory overhead**: ~100 bytes for basic metrics + small sample buffer
- **CPU overhead**: Background thread sleeps most of the time
- **Thread-safe**: Lock-free hot path using atomic operations

### Performance Tuning

```rust
// For high-performance applications
let config = TrackerConfig {
    sampling_interval_ms: 5000,  // Less frequent sampling
    growth_threshold_bytes_per_sec: 10 * 1024 * 1024, // Higher threshold
    leak_threshold_bytes: 100 * 1024 * 1024, // Higher threshold
    enable_backtrace: false, // Disable backtraces
}.validate()?;
```

## Features

- `backtrace`: Enable call site tracking for detailed allocation hotspots (requires `backtrace` crate)
- Default features provide leak and growth detection with minimal overhead

## Troubleshooting

### Common Issues

**No warnings appearing?**
- Ensure `init_tracker()` is called early in `main()`
- Check that your application actually allocates memory
- Try debug configuration for more sensitive detection

**Too many false positives?**
- Increase thresholds in configuration
- Use production configuration for conservative settings
- Extend sampling interval for less frequent checks

**High overhead?**
- Use production configuration
- Disable backtrace feature
- Increase sampling interval

### Getting Help

- Check stderr output for warnings and scope reports
- Use scoped tracking to narrow down problematic code
- Run examples to verify the library is working

## License

This project is licensed under the MIT License - see the LICENSE file for details.
