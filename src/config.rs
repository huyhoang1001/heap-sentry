/// Output format for memory reports
#[derive(Debug, Clone)]
pub enum OutputFormat {
    /// Output to stderr (default)
    Stderr,
    /// Output structured JSON to stderr
    JsonStderr,
    /// Output to a file (not implemented yet)
    File(String),
}

/// Configuration for the heap sentry tracker
#[derive(Debug)]
pub struct TrackerConfig {
    /// Sampling interval in milliseconds
    pub sampling_interval_ms: u64,
    /// Growth threshold in bytes per second
    pub growth_threshold_bytes_per_sec: usize,
    /// Leak threshold in bytes
    pub leak_threshold_bytes: usize,
    /// Enable backtrace collection for call sites
    pub enable_backtrace: bool,
    /// Backtrace sampling rate (1 in N allocations)
    pub backtrace_sample_rate: usize,
    /// Output format for reports
    pub output_format: OutputFormat,
}

impl Default for TrackerConfig {
    fn default() -> Self {
        Self {
            sampling_interval_ms: 1000,
            growth_threshold_bytes_per_sec: 1024 * 1024, // 1MB/s
            leak_threshold_bytes: 10 * 1024 * 1024, // 10MB
            enable_backtrace: false,
            backtrace_sample_rate: 100,
            output_format: OutputFormat::Stderr,
        }
    }
}

impl TrackerConfig {
    /// Validate configuration and return a validated config
    pub fn validate(self) -> Result<Self, String> {
        if self.sampling_interval_ms == 0 {
            return Err("sampling_interval_ms must be greater than 0".to_string());
        }
        if self.sampling_interval_ms > 3600000 { // 1 hour
            return Err("sampling_interval_ms should be less than 3600000 (1 hour)".to_string());
        }
        if self.growth_threshold_bytes_per_sec == 0 {
            return Err("growth_threshold_bytes_per_sec must be greater than 0".to_string());
        }
        if self.leak_threshold_bytes == 0 {
            return Err("leak_threshold_bytes must be greater than 0".to_string());
        }
        if self.backtrace_sample_rate == 0 {
            return Err("backtrace_sample_rate must be greater than 0".to_string());
        }
        // Validate output format
        match &self.output_format {
            OutputFormat::File(path) if path.is_empty() => {
                return Err("file path cannot be empty".to_string());
            }
            _ => {} // Other formats are valid
        }
        Ok(self)
    }

    /// Create configuration from environment variables
    /// Supports: HEAP_SENTRY_MODE, HEAP_SENTRY_INTERVAL, HEAP_SENTRY_GROWTH_THRESHOLD, HEAP_SENTRY_LEAK_THRESHOLD
    pub fn from_env() -> Result<Self, String> {
        let mut config = Self::default();

        // Check for mode override
        if let Ok(mode) = std::env::var("HEAP_SENTRY_MODE") {
            match mode.to_lowercase().as_str() {
                "debug" => config = Self::debug(),
                "production" => config = Self::production(),
                "development" => config = Self::debug(),
                _ => return Err(format!("Unknown HEAP_SENTRY_MODE: {}. Use 'debug' or 'production'", mode)),
            }
        }

        // Override individual settings if specified
        if let Ok(interval) = std::env::var("HEAP_SENTRY_INTERVAL") {
            config.sampling_interval_ms = interval.parse()
                .map_err(|_| format!("Invalid HEAP_SENTRY_INTERVAL: {}", interval))?;
        }

        if let Ok(growth) = std::env::var("HEAP_SENTRY_GROWTH_THRESHOLD") {
            config.growth_threshold_bytes_per_sec = growth.parse()
                .map_err(|_| format!("Invalid HEAP_SENTRY_GROWTH_THRESHOLD: {}", growth))?;
        }

        if let Ok(leak) = std::env::var("HEAP_SENTRY_LEAK_THRESHOLD") {
            config.leak_threshold_bytes = leak.parse()
                .map_err(|_| format!("Invalid HEAP_SENTRY_LEAK_THRESHOLD: {}", leak))?;
        }

        if let Ok(sample_rate) = std::env::var("HEAP_SENTRY_BACKTRACE_SAMPLE_RATE") {
            config.backtrace_sample_rate = sample_rate.parse()
                .map_err(|_| format!("Invalid HEAP_SENTRY_BACKTRACE_SAMPLE_RATE: {}", sample_rate))?;
        }

        config.validate()
    }

    /// Zero-config initialization - uses environment variables or defaults
    pub fn auto() -> Self {
        Self::from_env().unwrap_or_else(|_| Self::default())
    }

    /// Create a debug configuration with more sensitive settings
    pub fn debug() -> Self {
        Self {
            sampling_interval_ms: 500,
            growth_threshold_bytes_per_sec: 100 * 1024, // 100KB/s
            leak_threshold_bytes: 1024 * 1024, // 1MB
            enable_backtrace: true,
            backtrace_sample_rate: 20,
            output_format: OutputFormat::Stderr,
        }
    }

    /// Create a production configuration with conservative settings
    pub fn production() -> Self {
        Self {
            sampling_interval_ms: 5000, // Less frequent sampling
            growth_threshold_bytes_per_sec: 10 * 1024 * 1024, // 10MB/s
            leak_threshold_bytes: 100 * 1024 * 1024, // 100MB
            enable_backtrace: false, // Disable for performance
            backtrace_sample_rate: 100,
            output_format: OutputFormat::Stderr,
        }
    }
}