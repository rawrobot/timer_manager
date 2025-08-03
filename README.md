# File: /home/k14/work/pesonal/github/timer_manager/README.md
# Timer Manager

A high-performance, asynchronous timer management library for Rust built on top of Tokio. This library provides a robust way to manage multiple named timers with precise expiration handling and graceful shutdown capabilities.

## Features

- **Asynchronous Timer Management**: Built on Tokio for high-performance async operations
- **Named Timers**: Create and manage multiple timers with unique names
- **Bounded Channels**: Configurable buffer sizes for commands and events
- **Graceful Shutdown**: Clean shutdown with cancellation token support
- **Heartbeat-based Checking**: Configurable interval for timer expiration checks
- **Non-blocking Operations**: Both blocking and non-blocking API variants
- **Comprehensive Logging**: Built-in logging for debugging and monitoring

## Installation

Add this to your `Cargo.toml`:

```toml
[dependencies]
timer_manager = "0.1.0"
```

## Quick Start

```rust
use timer_manager::TimerManager;
use tokio_util::sync::CancellationToken;
use std::time::Duration;

#[tokio::main]
async fn main() {
    // Initialize logging (optional)
    env_logger::init();
    
    let cancel_token = CancellationToken::new();
    
    // Create timer manager with configuration
    let (manager, mut handle) = TimerManager::new(
        "my_timer_manager".to_string(),
        Duration::from_millis(10),  // heartbeat interval
        100,                        // command buffer size
        100,                        // event buffer size
        cancel_token.clone(),
    );

    // Spawn the manager task
    tokio::spawn(manager.run());

    // Set a timer
    handle.set_timer("my_timer".to_string(), Duration::from_secs(5)).await.unwrap();

    // Wait for timer expiration
    if let Some(event) = handle.recv_event().await {
        match event {
            TimerEvent::TimerExpired { name } => {
                println!("Timer '{}' expired!", name);
            }
        }
    }

    // Shutdown gracefully
    handle.shutdown().await.unwrap();
}
```

## API Reference

### TimerManager

The main timer management struct that runs the timer loop.

#### Constructor

```rust
pub fn new(
    name: String,
    heartbeat_interval: Duration,
    command_buffer_size: usize,
    event_buffer_size: usize,
    cancel_token: CancellationToken,
) -> (Self, TimerHandle)
```

**Parameters:**
- `name`: Instance name for logging purposes
- `heartbeat_interval`: How often to check for expired timers
- `command_buffer_size`: Buffer size for command channel
- `event_buffer_size`: Buffer size for event channel
- `cancel_token`: Token for graceful shutdown

**Returns:** A tuple containing the `TimerManager` and `TimerHandle`

#### Methods

```rust
pub async fn run(self)
```
Runs the timer manager loop. This should be spawned as a separate task.

### TimerHandle

Handle for interacting with the timer manager.

#### Timer Operations

```rust
// Set or update a timer (blocking)
pub async fn set_timer(&self, name: String, duration: Duration) -> Result<(), mpsc::error::SendError<TimerCommand>>

// Set or update a timer (non-blocking)
pub fn try_set_timer(&self, name: String, duration: Duration) -> Result<(), mpsc::error::TrySendError<TimerCommand>>

// Cancel a specific timer (blocking)
pub async fn cancel_timer(&self, name: String) -> Result<(), mpsc::error::SendError<TimerCommand>>

// Cancel a specific timer (non-blocking)
pub fn try_cancel_timer(&self, name: String) -> Result<(), mpsc::error::TrySendError<TimerCommand>>

// Cancel all timers (blocking)
pub async fn cancel_all_timers(&self) -> Result<(), mpsc::error::SendError<TimerCommand>>

// Cancel all timers (non-blocking)
pub fn try_cancel_all_timers(&self) -> Result<(), mpsc::error::TrySendError<TimerCommand>>
```

#### Event Handling

```rust
// Receive timer events (blocking)
pub async fn recv_event(&mut self) -> Option<TimerEvent>

// Try to receive timer events (non-blocking)
pub fn try_recv_event(&mut self) -> Result<TimerEvent, mpsc::error::TryRecvError>
```

#### Shutdown

```rust
// Shutdown the timer manager (blocking)
pub async fn shutdown(&self) -> Result<(), mpsc::error::SendError<TimerCommand>>

// Shutdown the timer manager (non-blocking)
pub fn try_shutdown(&self) -> Result<(), mpsc::error::TrySendError<TimerCommand>>
```

### TimerEvent

Events emitted by the timer manager.

```rust
pub enum TimerEvent {
    TimerExpired { name: String },
}
```

## Examples

### Basic Timer Usage

```rust
use timer_manager::{TimerManager, TimerEvent};
use tokio_util::sync::CancellationToken;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let cancel_token = CancellationToken::new();
    let (manager, mut handle) = TimerManager::new(
        "example".to_string(),
        Duration::from_millis(10),
        10,
        10,
        cancel_token,
    );

    tokio::spawn(manager.run());

    // Set multiple timers
    handle.set_timer("timer1".to_string(), Duration::from_secs(1)).await.unwrap();
    handle.set_timer("timer2".to_string(), Duration::from_secs(2)).await.unwrap();

    // Handle timer events
    while let Some(event) = handle.recv_event().await {
        match event {
            TimerEvent::TimerExpired { name } => {
                println!("Timer {} expired", name);
                break; // Exit after first timer
            }
        }
    }

    handle.shutdown().await.unwrap();
}
```

### Non-blocking Operations

```rust
use timer_manager::{TimerManager, TimerEvent};
use tokio_util::sync::CancellationToken;
use std::time::Duration;

#[tokio::main]
async fn main() {
    let cancel_token = CancellationToken::new();
    let (manager, mut handle) = TimerManager::new(
        "non_blocking_example".to_string(),
        Duration::from_millis(10),
        10,
        10,
        cancel_token,
    );

    tokio::spawn(manager.run());

    // Non-blocking timer operations
    match handle.try_set_timer("quick_timer".to_string(), Duration::from_millis(100)) {
        Ok(_) => println!("Timer set successfully"),
        Err(e) => println!("Failed to set timer: {}", e),
    }

    // Non-blocking event checking
    loop {
        match handle.try_recv_event() {
            Ok(TimerEvent::TimerExpired { name }) => {
                println!("Timer {} expired", name);
                break;
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Empty) => {
                // No events available, do other work
                tokio::time::sleep(Duration::from_millis(10)).await;
            }
            Err(tokio::sync::mpsc::error::TryRecvError::Disconnected) => {
                println!("Timer manager disconnected");
                break;
            }
        }
    }

    handle.try_shutdown().unwrap();
}
```

## Configuration

### Heartbeat Interval

The heartbeat interval determines how frequently the timer manager checks for expired timers. A shorter interval provides more precise timing but uses more CPU resources.

```rust
// High precision (more CPU usage)
Duration::from_millis(1)

// Balanced (recommended for most use cases)
Duration::from_millis(10)

// Lower precision (less CPU usage)
Duration::from_millis(100)
```

### Buffer Sizes

Configure channel buffer sizes based on your expected load:

```rust
// High-throughput scenario
let command_buffer_size = 1000;
let event_buffer_size = 1000;

// Low-throughput scenario
let command_buffer_size = 10;
let event_buffer_size = 10;
```

## Error Handling

The library uses standard Tokio MPSC error types:

- `mpsc::error::SendError<TimerCommand>`: Channel closed (manager stopped)
- `mpsc::error::TrySendError<TimerCommand>`: Channel full or closed
- `mpsc::error::TryRecvError`: No events available or channel closed

## Logging

The library uses the `log` crate for logging. Initialize a logger to see timer manager activity:

```rust
env_logger::init();
```

Log levels:
- `INFO`: Manager lifecycle events
- `DEBUG`: Timer operations (if implemented)
- `WARN`: Error conditions (if implemented)

## Performance Considerations

- Use appropriate heartbeat intervals for your precision requirements
- Size buffers according to your expected throughput
- Consider using non-blocking operations in high-performance scenarios
- The timer manager uses a single HashMap for timer storage, suitable for moderate numbers of concurrent timers

## License

This project is licensed under the MIT License - see the LICENSE file for details.

## Contributing

Contributions are welcome! Please feel free to submit a Pull Request.
