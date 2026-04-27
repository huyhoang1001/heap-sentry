use std::time::{Duration, Instant};

use crate::config::TrackerConfig;
use crate::metrics::METRICS;

/// Output a message according to the configured format
fn output_message(config: &TrackerConfig, level: &str, message: &str) {
    match &config.output_format {
        crate::config::OutputFormat::Stderr => {
            eprintln!("[{}] {}", level, message);
        }
        crate::config::OutputFormat::JsonStderr => {
            // Simple JSON format for now
            let timestamp = std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap_or_default().as_secs();
            eprintln!("{{\"level\":\"{}\",\"message\":\"{}\",\"timestamp\":{}}}",
                     level, message, timestamp);
        }
        crate::config::OutputFormat::File(_) => {
            // File output not implemented yet, fall back to stderr
            eprintln!("[{}] {} (file output not implemented)", level, message);
        }
    }
}

/// Sample of memory usage at a point in time
#[derive(Clone)]
pub struct Sample {
    pub timestamp: Instant,
    pub memory_usage: usize,
}

/// Initialize the heap sentry tracker with the given configuration.
/// This starts a background thread that monitors memory usage.
///
/// # Errors
///
/// Returns an error if the configuration is invalid.
pub fn init_tracker(config: TrackerConfig) -> Result<(), String> {
    let config = config.validate()?;
    super::allocator::ENABLE_BACKTRACE.store(config.enable_backtrace as usize, std::sync::atomic::Ordering::Relaxed);

    // Start sampling thread
    std::thread::spawn(move || {
        let mut samples: Vec<Sample> = Vec::new();
        let interval = Duration::from_millis(config.sampling_interval_ms);
        loop {
            std::thread::sleep(interval);
            let usage = METRICS.current_usage.load(std::sync::atomic::Ordering::Relaxed);
            let sample = Sample {
                timestamp: Instant::now(),
                memory_usage: usage,
            };
            samples.push(sample);
            // Keep only last 100 samples
            if samples.len() > 100 {
                samples.remove(0);
            }
            // Analyze
            analyze_and_report(&samples, &config);
        }
    });
    Ok(())
}

/// Initialize heap sentry with automatic configuration.
/// Uses environment variables if available, otherwise defaults.
/// This is the recommended way to initialize heap sentry for most use cases.
///
/// # Errors
///
/// Returns an error if the auto-configuration fails.
pub fn init() -> Result<(), String> {
    let config = crate::config::TrackerConfig::auto();
    init_tracker(config)
}

/// Analyze samples and report issues
fn analyze_and_report(samples: &[Sample], config: &TrackerConfig) {
    if samples.len() < 2 {
        return;
    }

    let total_allocated = METRICS.total_allocated.load(std::sync::atomic::Ordering::Relaxed);
    let total_freed = METRICS.total_freed.load(std::sync::atomic::Ordering::Relaxed);
    let leak_size = total_allocated.saturating_sub(total_freed);

    if leak_size > config.leak_threshold_bytes {
        // Check if not decreasing over last 10 samples
        let start = samples.len().saturating_sub(10);
        let recent = &samples[start..];
        let decreasing = recent.windows(2).all(|w| w[1].memory_usage <= w[0].memory_usage);
        if !decreasing {
            output_message(config, "WARN", &format!("Potential memory leak detected: {} bytes not freed", leak_size));
            output_message(config, "INFO", "Use scoped tracking or enable the backtrace feature to pinpoint the leaking code path.");
        }
    }

    // Growth detection
    let first = &samples[0];
    let last = &samples[samples.len() - 1];
    let time_diff = last.timestamp.duration_since(first.timestamp).as_secs_f64();
    if time_diff > 0.0 {
        let growth = last.memory_usage as f64 - first.memory_usage as f64;
        let rate = growth / time_diff;
        if rate > config.growth_threshold_bytes_per_sec as f64 {
            output_message(config, "WARN", &format!("Unbounded memory growth detected: {:.2} bytes/sec", rate));
            output_message(config, "INFO", "Use scoped tracking or enable the backtrace feature to identify which code paths are growing.");
        }
    }

    // Hotspots
    #[cfg(feature = "backtrace")]
    if config.enable_backtrace {
        if let Ok(map) = METRICS.callsites.lock() {
            let mut hotspots: Vec<_> = map.iter().collect();
            hotspots.sort_by(|a, b| b.1.allocated.cmp(&a.1.allocated));
            if !hotspots.is_empty() {
                output_message(config, "INFO", "Top allocation sources:");
                for (callsite, stats) in hotspots.iter().take(5) {
                    output_message(config, "INFO", &format!("  - {}: {} bytes ({} allocations)", callsite, stats.allocated, stats.count));
                }
            }
        } else {
            output_message(config, "WARN", "Could not access callsite data due to mutex poisoning");
        }
    }
}