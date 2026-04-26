use heap_sentry::{init_tracker, TrackerConfig};
use std::thread;
use std::time::Duration;

fn main() {
    // Note: Compile with --features backtrace to enable this
    let config = TrackerConfig {
        sampling_interval_ms: 2000,
        growth_threshold_bytes_per_sec: 1024 * 1024,
        leak_threshold_bytes: 5 * 1024 * 1024,
        enable_backtrace: true, // Requires --features backtrace
    };

    init_tracker(config);

    println!("Starting backtrace example...");
    println!("This example enables call site tracking.");
    println!("Compile with: cargo run --example backtrace_example --features backtrace");

    // Simulate some allocations from different functions
    allocate_from_main();
    allocate_from_helper();

    // Create a leak
    let _leaked = vec![0u8; 2 * 1024 * 1024]; // 2MB leak

    println!("Waiting for analysis...");
    thread::sleep(Duration::from_secs(5));

    println!("Example completed. Check for hotspot reports.");
}

fn allocate_from_main() {
    let _data = vec![0u8; 1024 * 1024]; // 1MB
    thread::sleep(Duration::from_millis(100));
}

fn allocate_from_helper() {
    let _data = vec![0u8; 512 * 1024]; // 512KB
    thread::sleep(Duration::from_millis(100));
}