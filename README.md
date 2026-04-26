# heap-sentry
HeapSentry is a lightweight Rust library for tracking heap allocations, detecting memory leaks, and identifying unbounded memory growth. It provides real-time metrics and actionable insights with low overhead in production and deeper diagnostics in debug mode.

## Documentation

- **[How It Works](docs/how_it_works.md)** - Detailed explanation of the architecture, algorithms, and internal workings

## Usage

Add to your `Cargo.toml`:

```toml
[dependencies]
heap-sentry = "0.1"
```

In your code:

```rust
use heap_sentry::{init_tracker, TrackerConfig};

fn main() {
    init_tracker(TrackerConfig::default());

    loop {
        let data = vec![0u8; 1024 * 1024]; // simulate allocation
        std::thread::sleep(std::time::Duration::from_millis(100));
    }
}
```

The library will automatically track allocations and report warnings to stderr.

## Configuration

```rust
let config = TrackerConfig {
    sampling_interval_ms: 1000,
    growth_threshold_bytes_per_sec: 1024 * 1024,
    leak_threshold_bytes: 10 * 1024 * 1024,
    enable_backtrace: false,
};
```

## Running the Examples

To see the library in action, run the included examples:

```bash
# Basic leak detection
cargo run --example leak_example

# Growth rate detection with custom config
cargo run --example growth_example

# Call site tracking (requires backtrace feature)
cargo run --example backtrace_example --features backtrace
```

These examples demonstrate different aspects of memory monitoring.

## Features

- `backtrace`: Enable call site tracking (requires `backtrace` crate).
