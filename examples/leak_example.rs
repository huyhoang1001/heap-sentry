use heap_sentry::{init_tracker, TrackerConfig};
use std::thread;
use std::time::Duration;

fn main() {
    // Initialize the tracker with default config
    init_tracker(TrackerConfig::default());

    println!("Starting memory tracking example...");

    // Simulate bounded memory usage (should not trigger warnings)
    println!("Phase 1: Bounded allocations");
    for i in 0..10 {
        let _data = vec![0u8; 1024 * 1024]; // 1MB allocation, but dropped immediately
        thread::sleep(Duration::from_millis(500));
        println!("Iteration {}: Allocated and freed 1MB", i + 1);
    }

    // Simulate unbounded growth (should trigger growth warning)
    println!("Phase 2: Unbounded growth");
    let mut allocations = Vec::new();
    for i in 0..20 {
        allocations.push(vec![0u8; 512 * 1024]); // 512KB, kept in vector
        thread::sleep(Duration::from_millis(500));
        println!("Iteration {}: Retained 512KB, total retained: {} KB", i + 1, allocations.len() * 512);
    }

    // Wait a bit to let the analyzer detect the growth
    thread::sleep(Duration::from_secs(5));

    println!("Example completed. Check stderr for any warnings.");
}