//! # Timer Manager
//!
//! A high-performance, asynchronous timer management library for Rust built on top of Tokio.
//!
//! This library provides a simple yet powerful way to manage multiple named timers with precise
//! expiration handling and supports graceful shutdowns.
//!
//! ## Features
//!
//! - **Asynchronous**: Built on Tokio for high-performance async operations
//! - **Named Timers**: Manage multiple timers with string identifiers
//! - **Bounded Channels**: Configurable buffer sizes for command and event handling
//! - **Graceful Shutdown**: Support for cancellation tokens and clean shutdowns
//! - **Non-blocking Operations**: Both blocking and non-blocking timer operations
//! - **Comprehensive Logging**: Built-in logging for debugging and monitoring
//!
//! ## Quick Start
//!
//! ```rust
//! use timer_manager::TimerManager;
//! use tokio_util::sync::CancellationToken;
//! use std::time::Duration;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let cancel_token = CancellationToken::new();
//!     
//!     // Create timer manager with configuration
//!     let (manager, mut handle) = TimerManager::new(
//!         "my_timer_manager".to_string(),
//!         Duration::from_millis(10),  // heartbeat interval
//!         100,                        // command buffer size
//!         100,                        // event buffer size
//!         cancel_token.clone(),
//!     );
//!
//!     // Spawn the manager task
//!     tokio::spawn(manager.run());
//!
//!     // Set a timer
//!     handle.set_timer("my_timer".to_string(), Duration::from_secs(1)).await?;
//!
//!     // Wait for timer expiration
//!     if let Some(event) = handle.recv_event().await {
//!         match event {
//!             timer_manager::TimerEvent::TimerExpired { name } => {
//!                 println!("Timer '{}' expired!", name);
//!             }
//!         }
//!     }
//!
//!     // Shutdown gracefully
//!     handle.shutdown().await?;
//!     Ok(())
//! }
//! ```

mod tm;

pub use tm::{TimerCommand, TimerEvent, TimerHandle, TimerManager};

// Re-export commonly used types for convenience
pub use std::time::Duration;
pub use tokio_util::sync::CancellationToken;
