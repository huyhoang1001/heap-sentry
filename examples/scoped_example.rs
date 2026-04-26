use heap_sentry::{init_tracker, TrackerConfig, track_scope};
use std::thread;
use std::time::Duration;

fn main() {
    // Initialize with debug config for more sensitive monitoring
    init_tracker(TrackerConfig::debug()).expect("Failed to initialize tracker");

    println!("Starting scoped tracking example...");
    println!("This demonstrates memory tracking for specific code blocks.");

    // Example 1: Manual scope tracking
    println!("\n=== Manual Scope Tracking ===");
    {
        let scope = heap_sentry::MemoryScope::new("manual_scope");
        let _data = vec![0u8; 2 * 1024 * 1024]; // 2MB
        thread::sleep(Duration::from_millis(100));
        let stats = scope.stats();
        println!("Manual scope stats: {} bytes allocated", stats.allocated);
    }

    // Example 2: Macro-based scope tracking
    println!("\n=== Macro-Based Scope Tracking ===");
    track_scope!("macro_scope", {
        let _data1 = vec![0u8; 1024 * 1024]; // 1MB
        thread::sleep(Duration::from_millis(50));
        track_scope!("nested_scope", {
            let _data2 = vec![0u8; 512 * 1024]; // 512KB
            thread::sleep(Duration::from_millis(50));
        });
        let _data3 = vec![0u8; 1024 * 1024]; // Another 1MB
    });

    // Example 3: Function-level tracking simulation
    println!("\n=== Function-Level Tracking ===");
    process_large_dataset();
    process_small_dataset();

    println!("\nWaiting for analysis...");
    thread::sleep(Duration::from_secs(3));

    println!("Example completed. Check stderr for scope reports and any warnings.");
}

fn process_large_dataset() {
    let _scope = heap_sentry::MemoryScope::new("process_large_dataset");
    let _dataset = vec![0u8; 5 * 1024 * 1024]; // 5MB
    // Simulate processing
    thread::sleep(Duration::from_millis(200));
    // Data is dropped here when scope ends
}

fn process_small_dataset() {
    let _scope = heap_sentry::MemoryScope::new("process_small_dataset");
    let _dataset = vec![0u8; 100 * 1024]; // 100KB
    // Simulate processing
    thread::sleep(Duration::from_millis(50));
    // Data is dropped here when scope ends
}