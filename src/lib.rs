//! # Heap Sentry
//!
//! A lightweight Rust library for detecting memory leaks and unbounded memory growth.
//!
//! ## Features
//!
//! - **Memory Leak Detection**: Identifies when allocated memory exceeds freed memory by a threshold
//! - **Growth Monitoring**: Detects sustained memory growth rates above configurable limits
//! - **Call Site Tracking**: Optional backtrace collection to identify allocation hotspots
//! - **Low Overhead**: Uses atomic operations and background sampling for minimal performance impact
//!
//! ## Usage
//!
//! ```rust
//! use heap_sentry::{init_tracker, TrackerConfig};
//!
//! fn main() {
//!     init_tracker(TrackerConfig::default());
//!     // Your application code here
//! }
//! ```

pub mod config;
pub mod metrics;
pub mod allocator;
pub mod analysis;
pub mod scope;

// Re-export public API
pub use config::TrackerConfig;
pub use metrics::{snapshot, MemoryStats, ThreadMemoryStats, thread_stats, current_thread_stats};
pub use analysis::init_tracker;
pub use scope::{MemoryScope, ScopedStats};