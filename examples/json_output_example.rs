use heap_sentry::{init_tracker, TrackerConfig, config::OutputFormat};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Configure for JSON output
    let config = TrackerConfig {
        sampling_interval_ms: 500,
        growth_threshold_bytes_per_sec: 100 * 1024, // 100KB/s - sensitive for demo
        leak_threshold_bytes: 1024 * 1024, // 1MB
        enable_backtrace: false,
        output_format: OutputFormat::JsonStderr,
    }.validate()?;

    let _ = init_tracker(config);

    // Allocate memory to trigger growth detection
    for _ in 0..50 {
        let data = vec![0u8; 10 * 1024]; // 10KB per iteration
        std::thread::sleep(std::time::Duration::from_millis(10));
        // Don't drop data to create growth
        std::mem::forget(data);
    }

    std::thread::sleep(std::time::Duration::from_secs(2)); // Wait for analysis
    Ok(())
}