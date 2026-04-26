use heap_sentry::{init_tracker, TrackerConfig, thread_stats};
use std::thread;
use std::time::Duration;

fn main() {
    // Use debug config for more sensitive detection
    init_tracker(TrackerConfig::debug()).expect("Failed to initialize tracker");

    println!("Starting multi-threaded workflow example...");

    // Spawn a few worker threads
    let mut handles = vec![];

    for i in 0..3 {
        let handle = thread::Builder::new()
            .name(format!("worker-{}", i))
            .spawn(move || {
                run_worker(i);
            })
            .expect("Failed to spawn thread");
        handles.push(handle);
    }

    // Main thread work
    println!("Main thread: Starting work");
    let _data = vec![0u8; 1024 * 1024]; // 1MB
    thread::sleep(Duration::from_millis(1000));

    // Wait for threads
    for handle in handles {
        handle.join().unwrap();
    }

    // Report thread statistics
    println!("\n=== Thread Statistics ===");
    let all_thread_stats = thread_stats();
    for stats in all_thread_stats {
        if stats.current_usage > 0 {
            println!("Thread {}: {} KB used, {} allocs",
                    stats.thread_name,
                    stats.current_usage / 1024,
                    stats.allocation_count);
        }
    }

    println!("\nWaiting for analysis...");
    thread::sleep(Duration::from_secs(3));

    println!("Example completed.");
}

fn run_worker(id: usize) {
    println!("Worker {}: Starting", id);

    // Simulate different memory patterns
    match id {
        0 => {
            // Worker 0: Allocate and free
            for _ in 0..10 {
                let _temp = vec![0u8; 10 * 1024]; // 10KB
                thread::sleep(Duration::from_millis(100));
            }
        }
        1 => {
            // Worker 1: Keep some allocations
            let mut kept = vec![];
            for i in 0..20 {
                let data = vec![0u8; 5 * 1024]; // 5KB
                if i % 2 == 0 {
                    kept.push(data); // Keep half
                }
                thread::sleep(Duration::from_millis(50));
            }
            println!("Worker 1: Kept {} allocations", kept.len());
            // Keep the data
            std::mem::forget(kept);
        }
        2 => {
            // Worker 2: Heavy allocations
            let _big_data = vec![0u8; 500 * 1024]; // 500KB
            thread::sleep(Duration::from_millis(500));
        }
        _ => {}
    }

    println!("Worker {}: Finished", id);
}