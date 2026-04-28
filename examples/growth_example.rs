use heap_sentry::{init_tracker, TrackerConfig, snapshot};
use std::thread;
use std::time::Duration;

fn main() {
    // Configure for faster detection
    let config = TrackerConfig {
        sampling_interval_ms: 500, // Sample every 0.5s
        growth_threshold_bytes_per_sec: 500 * 1024, // 500KB/s threshold
        leak_threshold_bytes: 50 * 1024 * 1024, // Higher leak threshold
        enable_backtrace: false,
        output_format: heap_sentry::config::OutputFormat::Stderr,
    };

    let _ = init_tracker(config);

    println!("Starting growth detection example...");
    println!("This will allocate memory rapidly to trigger growth warnings.");

    let mut allocations = Vec::new();

    for i in 0..50 {
        // Allocate 100KB every 100ms
        allocations.push(vec![0u8; 100 * 1024]);
        thread::sleep(Duration::from_millis(100));

        if i % 10 == 0 {
            let stats = snapshot();
            println!("Iteration {}: Current usage: {} KB, Total allocated: {} KB",
                     i + 1,
                     stats.current_usage / 1024,
                     stats.total_allocated / 1024);
        }
    }

    println!("Waiting for final analysis...");
    thread::sleep(Duration::from_secs(3));

    println!("Example completed.");
}