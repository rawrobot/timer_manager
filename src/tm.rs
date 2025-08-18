use std::collections::HashMap;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tokio::time::{interval, MissedTickBehavior};
use tokio_util::sync::CancellationToken;

/// Simple Timer Manager for FSM communication
pub struct TimerManager {
    /// Instance name for logging
    name: String,

    /// Channel for receiving timer commands
    command_rx: mpsc::Receiver<TimerCommand>,

    /// Channel for sending timer events
    event_tx: mpsc::Sender<TimerEvent>,

    /// Timer storage: timer_name -> expiration_time
    timers: HashMap<String, Instant>,

    /// Heartbeat interval for timer checks
    heartbeat_interval: Duration,
    //// Cancellation token for graceful shutdown
    cancel_token: CancellationToken,
}

/// Handle for controlling the timer manager
pub struct TimerHandle {
    /// Channel for sending commands to the timer manager
    command_tx: mpsc::Sender<TimerCommand>,

    /// Channel for receiving timer events
    event_rx: mpsc::Receiver<TimerEvent>,
}

/// Timer command enum
#[derive(Debug, Clone)]
pub enum TimerCommand {
    SetTimer { name: String, duration: Duration },
    CancelTimer { name: String },
    CancelAllTimers,
    Shutdown,
}

/// Timer event enum
#[derive(Debug, Clone)]
pub enum TimerEvent {
    TimerExpired { name: String },
}

impl TimerManager {
    /// Create a new TimerManager with bounded channels
    ///
    /// # Arguments
    /// * `name` - Timer manager instance name
    /// * `heartbeat_interval` - How often to check for expired timers
    /// * `command_buffer_size` - Size of command channel buffer
    /// * `event_buffer_size` - Size of event channel buffer
    ///
    /// Returns (TimerManager, TimerHandle)
    pub fn new(
        name: String,
        heartbeat_interval: Duration,
        command_buffer_size: usize,
        event_buffer_size: usize,
        cancel_token: CancellationToken,
    ) -> (Self, TimerHandle) {
        let (command_tx, command_rx) = mpsc::channel(command_buffer_size);
        let (event_tx, event_rx) = mpsc::channel(event_buffer_size);

        let manager = TimerManager {
            name,
            command_rx,
            event_tx,
            timers: HashMap::new(),
            heartbeat_interval,
            cancel_token,
        };

        let handle = TimerHandle {
            command_tx,
            event_rx,
        };

        (manager, handle)
    }

    /// Run the timer manager
    pub async fn run(mut self) {
        let mut heartbeat = interval(self.heartbeat_interval);
        heartbeat.set_missed_tick_behavior(MissedTickBehavior::Skip);

        log::info!("Timer manager '{}' started", self.name);

        loop {
            tokio::select! {
                // Handle incoming commands
                Some(command) = self.command_rx.recv() => {
                    match command {
                        _ if self.cancel_token.is_cancelled() => {
                            log::info!("Timer manager '{}' cancelled", self.name);
                            break;
                        }
                        _ => {
                            let shutdown = self.handle_command(command).await;
                            if shutdown {
                                break;
                            }
                        }
                    }
                },

                // Check for expired timers
                _ = heartbeat.tick() => {
                    self.check_expired_timers().await;
                },

                // Handle cancellation token
                _ = self.cancel_token.cancelled() => {
                    log::info!("Timer manager '{}' cancelled via token", self.name);
                    break;
                },

                // All senders dropped
                else => {
                    log::info!("Timer manager '{}' shutting down - all senders dropped", self.name);
                    break;
                }
            }
        }

        log::info!("Timer manager '{}' stopped", self.name);
    }

    /// Handle timer commands
    async fn handle_command(&mut self, command: TimerCommand) -> bool {
        let mut shutdown = false;
        match command {
            TimerCommand::SetTimer { name, duration } => {
                let expires_at = Instant::now() + duration;
                let _was_replaced = self.timers.insert(name.clone(), expires_at).is_some();

                // if was_replaced {
                //     log::debug!("Timer '{}' updated in manager '{}'", name, self.name);
                // } else {
                //     log::debug!("Timer '{}' set in manager '{}' to expire in {:?}", name, self.name, duration);
                // }
            }
            TimerCommand::CancelTimer { name } => {
                if self.timers.remove(&name).is_some() {
                    //log::debug!("Timer '{}' canceled in manager '{}'", name, self.name);
                }
            }
            TimerCommand::CancelAllTimers => {
                //let count = self.timers.len();
                self.timers.clear();
                //log::debug!("Canceled all {} timer(s) in manager '{}'", count, self.name);
            }
            TimerCommand::Shutdown => {
                //log::info!("Timer manager '{}' shutting down", self.name);
                shutdown = true;
            }
        }
        shutdown
    }

    /// Check for expired timers and fire them
    async fn check_expired_timers(&mut self) {
        let now = Instant::now();
        let mut expired_timers = Vec::new();

        // Collect expired timers
        for (name, expires_at) in &self.timers {
            if *expires_at <= now {
                expired_timers.push(name.clone());
            }
        }

        // Process expired timers
        for name in expired_timers {
            // Remove from storage
            self.timers.remove(&name);

            // Send expiration event
            //log::debug!("Timer '{}' expired in manager '{}'", name, self.name);

            // Use try_send to avoid blocking if event channel is full
            if let Err(e) = self
                .event_tx
                .try_send(TimerEvent::TimerExpired { name: name.clone() })
            {
                match e {
                    mpsc::error::TrySendError::Full(_) => {
                        log::warn!(
                            "Event channel full, dropping timer expiration for '{}'",
                            name
                        );
                    }
                    mpsc::error::TrySendError::Closed(_) => {
                        log::warn!(
                            "Event channel closed, cannot send timer expiration for '{}'",
                            name
                        );
                        break;
                    }
                }
            }
        }
    }
}

impl TimerHandle {
    /// Set a timer (creates new or updates existing)
    pub async fn set_timer(
        &self,
        name: String,
        duration: Duration,
    ) -> Result<(), mpsc::error::SendError<TimerCommand>> {
        self.command_tx
            .send(TimerCommand::SetTimer { name, duration })
            .await
    }

    /// Set a timer (non-blocking)
    pub fn try_set_timer(
        &self,
        name: String,
        duration: Duration,
    ) -> Result<(), mpsc::error::TrySendError<TimerCommand>> {
        self.command_tx
            .try_send(TimerCommand::SetTimer { name, duration })
    }

    /// Cancel a specific timer
    pub async fn cancel_timer(
        &self,
        name: String,
    ) -> Result<(), mpsc::error::SendError<TimerCommand>> {
        self.command_tx
            .send(TimerCommand::CancelTimer { name })
            .await
    }

    /// Cancel a specific timer (non-blocking)
    pub fn try_cancel_timer(
        &self,
        name: String,
    ) -> Result<(), mpsc::error::TrySendError<TimerCommand>> {
        self.command_tx.try_send(TimerCommand::CancelTimer { name })
    }

    /// Cancel all timers
    pub async fn cancel_all_timers(&self) -> Result<(), mpsc::error::SendError<TimerCommand>> {
        self.command_tx.send(TimerCommand::CancelAllTimers).await
    }

    /// Cancel all timers (non-blocking)
    pub fn try_cancel_all_timers(&self) -> Result<(), mpsc::error::TrySendError<TimerCommand>> {
        self.command_tx.try_send(TimerCommand::CancelAllTimers)
    }

    /// Shutdown the timer manager
    pub async fn shutdown(&self) -> Result<(), mpsc::error::SendError<TimerCommand>> {
        self.command_tx.send(TimerCommand::Shutdown).await
    }

    /// Shutdown the timer manager (non-blocking)
    pub fn try_shutdown(&self) -> Result<(), mpsc::error::TrySendError<TimerCommand>> {
        self.command_tx.try_send(TimerCommand::Shutdown)
    }

    /// Receive the next timer event (blocking)
    pub async fn recv_event(&mut self) -> Option<TimerEvent> {
        self.event_rx.recv().await
    }

    /// Try to receive a timer event (non-blocking)
    pub fn try_recv_event(&mut self) -> Result<TimerEvent, mpsc::error::TryRecvError> {
        self.event_rx.try_recv()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tokio::time::sleep;

    #[tokio::test]
    async fn test_timer_basic_functionality() {
        let cancel_token = CancellationToken::new();
        let (manager, mut handle) = TimerManager::new(
            "test".to_string(),
            Duration::from_millis(10),
            10, // command buffer size
            10, // event buffer size
            cancel_token.clone(),
        );

        // Spawn the manager
        tokio::spawn(manager.run());

        // Set a short timer
        handle
            .set_timer("test_timer".to_string(), Duration::from_millis(50))
            .await
            .unwrap();

        // Wait for expiration
        let event = handle.recv_event().await.unwrap();
        match event {
            TimerEvent::TimerExpired { name } => {
                assert_eq!(name, "test_timer");
            }
        }

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_timer_cancel() {
        let cancel_token = CancellationToken::new();
        let (manager, mut handle) = TimerManager::new(
            "test".to_string(),
            Duration::from_millis(10),
            10,
            10,
            cancel_token.clone(),
        );

        tokio::spawn(manager.run());

        // Set a timer
        handle
            .set_timer("test_timer".to_string(), Duration::from_millis(100))
            .await
            .unwrap();

        // Cancel it immediately
        handle.cancel_timer("test_timer".to_string()).await.unwrap();

        // Wait a bit to ensure it doesn't fire
        sleep(Duration::from_millis(150)).await;

        // Should not receive any events
        assert!(handle.try_recv_event().is_err());

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_bounded_channel_backpressure() {
        let cancel_token = CancellationToken::new();
        let (manager, handle) = TimerManager::new(
            "test".to_string(),
            Duration::from_millis(10),
            2, // small command buffer
            2, // small event buffer
            cancel_token.clone(),
        );

        tokio::spawn(manager.run());

        // Fill up the command channel
        handle
            .set_timer("timer1".to_string(), Duration::from_millis(50))
            .await
            .unwrap();
        handle
            .set_timer("timer2".to_string(), Duration::from_millis(50))
            .await
            .unwrap();

        // This should work with try_send
        let _result = handle.try_set_timer("timer3".to_string(), Duration::from_millis(50));
        // Might succeed or fail depending on timing, but shouldn't panic

        handle.shutdown().await.unwrap();
    }

    #[tokio::test]
    async fn test_cancellation_token() {
        let cancel_token = CancellationToken::new();
        let (manager, mut handle) = TimerManager::new(
            "test".to_string(),
            Duration::from_millis(10),
            10,
            10,
            cancel_token.clone(),
        );

        let manager_task = tokio::spawn(manager.run());

        // Set a timer that should expire
        handle
            .set_timer("timer1".to_string(), Duration::from_millis(50))
            .await
            .unwrap();

        // Cancel the token
        cancel_token.cancel();

        // Wait for the manager to shut down
        let _ = manager_task.await;

        // Try to set a timer after cancellation - this should fail
        let result = handle.try_set_timer("timer2".to_string(), Duration::from_millis(50));
        assert!(
            result.is_err(),
            "Setting timer after cancellation should fail"
        );

        // Should not receive any timer events since manager was cancelled
        assert!(
            handle.try_recv_event().is_err(),
            "Should not receive events after cancellation"
        );
    }

    #[tokio::test]
    async fn test_cancellation_during_timer_operation() {
        let cancel_token = CancellationToken::new();
        let (manager, mut handle) = TimerManager::new(
            "test".to_string(),
            Duration::from_millis(10),
            10,
            10,
            cancel_token.clone(),
        );

        let manager_task = tokio::spawn(manager.run());

        // Set multiple timers
        handle
            .set_timer("timer1".to_string(), Duration::from_millis(100))
            .await
            .unwrap();
        handle
            .set_timer("timer2".to_string(), Duration::from_millis(200))
            .await
            .unwrap();

        // Wait a bit to ensure timers are set
        sleep(Duration::from_millis(20)).await;

        // Cancel the token before timers expire
        cancel_token.cancel();

        // Wait for the manager to shut down
        let _ = manager_task.await;

        // Timers should not fire since manager was cancelled
        assert!(
            handle.try_recv_event().is_err(),
            "No timer events should be received after cancellation"
        );

        // Subsequent operations should fail
        let result = handle.try_set_timer("timer3".to_string(), Duration::from_millis(50));
        assert!(result.is_err(), "Operations after cancellation should fail");
    }
}
