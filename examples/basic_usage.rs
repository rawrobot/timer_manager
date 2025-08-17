//! Basic usage example for the timer manager

use timer_manager::{CancellationToken, Duration, TimerEvent, TimerManager};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    env_logger::init();

    let cancel_token = CancellationToken::new();

    // Create timer manager with configuration
    let (manager, mut handle) = TimerManager::new(
        "example_timer_manager".to_string(),
        Duration::from_millis(10), // heartbeat interval
        100,                       // command buffer size
        100,                       // event buffer size
        cancel_token.clone(),
    );

    // Spawn the manager task
    let manager_task = tokio::spawn(manager.run());

    // Set multiple timers
    handle
        .set_timer("short_timer".to_string(), Duration::from_secs(1))
        .await?;
    handle
        .set_timer("medium_timer".to_string(), Duration::from_secs(2))
        .await?;
    handle
        .set_timer("long_timer".to_string(), Duration::from_secs(3))
        .await?;

    println!("Timers set! Waiting for expiration...");

    // Wait for timer expirations
    let mut expired_count = 0;
    while expired_count < 3 {
        if let Some(event) = handle.recv_event().await {
            match event {
                TimerEvent::TimerExpired { name } => {
                    println!("Timer '{}' expired!", name);
                    expired_count += 1;
                }
            }
        }
    }

    // Demonstrate cancellation
    handle
        .set_timer("cancelled_timer".to_string(), Duration::from_secs(10))
        .await?;
    println!("Set a timer that will be cancelled...");

    tokio::time::sleep(Duration::from_millis(100)).await;
    handle.cancel_timer("cancelled_timer".to_string()).await?;
    println!("Timer cancelled!");

    // Shutdown gracefully
    handle.shutdown().await?;
    manager_task.await?;

    println!("Timer manager shut down successfully!");
    Ok(())
}
