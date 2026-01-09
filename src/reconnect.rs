//! Auto-reconnection with exponential backoff

use crate::error::Result;
use crate::state::ConnectionStateMachine;
use crate::types::TunnelState;
use std::sync::Arc;
use std::time::Duration;
use tokio::time::sleep;

/// Reconnection configuration
#[derive(Debug, Clone)]
pub struct ReconnectConfig {
    /// Initial delay before first reconnection attempt
    pub initial_delay: Duration,
    /// Maximum delay between attempts (exponential backoff caps here)
    pub max_delay: Duration,
    /// Maximum number of reconnection attempts before giving up
    pub max_attempts: u32,
    /// Multiplier for exponential backoff (e.g., 2.0 = double each time)
    pub backoff_multiplier: f32,
}

impl Default for ReconnectConfig {
    fn default() -> Self {
        Self {
            initial_delay: Duration::from_secs(1),
            max_delay: Duration::from_secs(60),
            max_attempts: 10,
            backoff_multiplier: 2.0,
        }
    }
}

/// Reconnection manager
///
/// Handles exponential backoff and reconnection attempts
pub struct ReconnectManager {
    config: ReconnectConfig,
    current_delay: Duration,
    attempts: u32,
}

impl ReconnectManager {
    /// Create a new reconnection manager
    pub fn new(config: ReconnectConfig) -> Self {
        Self {
            current_delay: config.initial_delay,
            attempts: 0,
            config,
        }
    }

    /// Wait for next reconnection attempt
    ///
    /// Returns false if max attempts reached
    pub async fn wait_for_retry(&mut self) -> bool {
        if self.attempts >= self.config.max_attempts {
            tracing::error!(
                "Max reconnection attempts ({}) reached",
                self.config.max_attempts
            );
            return false;
        }

        self.attempts += 1;

        tracing::info!(
            "Reconnection attempt {}/{} in {:?}",
            self.attempts,
            self.config.max_attempts,
            self.current_delay
        );

        sleep(self.current_delay).await;

        // Exponential backoff
        self.current_delay = std::cmp::min(
            Duration::from_secs_f32(
                self.current_delay.as_secs_f32() * self.config.backoff_multiplier
            ),
            self.config.max_delay,
        );

        true
    }

    /// Reset after successful connection
    pub fn reset(&mut self) {
        self.attempts = 0;
        self.current_delay = self.config.initial_delay;
    }

    /// Get current attempt count
    pub fn attempts(&self) -> u32 {
        self.attempts
    }
}

/// Proxy for Tunnel to avoid circular dependency
pub struct TunnelProxy {
    /// Connection state machine for monitoring and state transitions
    pub state_machine: ConnectionStateMachine,
}

impl TunnelProxy {
    /// Internal reconnection method
    ///
    /// Called by the reconnection task when attempting to reconnect.
    /// This is a placeholder - actual implementation is in the main Tunnel.
    pub async fn reconnect_internal(&self) -> Result<()> {
        // TODO: Implement actual reconnection logic in main Tunnel
        tracing::info!("Attempting reconnection...");
        Ok(())
    }
}

/// Spawn reconnection task
///
/// Monitors connection state and attempts reconnection when disconnected
pub fn spawn_reconnect_task(
    tunnel: Arc<TunnelProxy>,
    config: ReconnectConfig,
) -> tokio::task::JoinHandle<()> {
    tokio::spawn(async move {
        let mut manager = ReconnectManager::new(config);
        let mut state_rx = tunnel.state_machine.subscribe();

        loop {
            // Wait for state change
            let _ = state_rx.changed().await;

            let state = state_rx.borrow().clone();

            match state {
                TunnelState::Connected { .. } => {
                    manager.reset();
                }
                TunnelState::Reconnecting { .. } | TunnelState::Failed { .. } => {
                    if manager.wait_for_retry().await {
                        // Attempt reconnection
                        match tunnel.reconnect_internal().await {
                            Ok(_) => {
                                tracing::info!("Reconnection successful");
                                manager.reset();
                            }
                            Err(e) => {
                                tracing::warn!("Reconnection failed: {}", e);
                            }
                        }
                    } else {
                        // Max attempts reached, transition to failed
                        tunnel.state_machine.transition(TunnelState::Failed {
                            error: "Max reconnection attempts exceeded".to_string(),
                            at: std::time::Instant::now(),
                        });
                    }
                }
                _ => {}
            }
        }
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_reconnect_backoff() {
        let config = ReconnectConfig {
            initial_delay: Duration::from_millis(10),
            max_delay: Duration::from_millis(100),
            max_attempts: 5,
            backoff_multiplier: 2.0,
        };

        let mut manager = ReconnectManager::new(config);

        // First retry: 10ms
        let start = std::time::Instant::now();
        assert!(manager.wait_for_retry().await);
        assert!(start.elapsed() >= Duration::from_millis(10));

        // Second retry: 20ms
        let start = std::time::Instant::now();
        assert!(manager.wait_for_retry().await);
        assert!(start.elapsed() >= Duration::from_millis(20));

        // Continue until max
        for _ in 0..3 {
            manager.wait_for_retry().await;
        }

        // Should return false after max attempts
        assert!(!manager.wait_for_retry().await);
    }

    #[test]
    fn test_reset() {
        let config = ReconnectConfig::default();
        let mut manager = ReconnectManager::new(config);

        // Simulate some attempts
        manager.attempts = 5;
        manager.current_delay = Duration::from_secs(30);

        manager.reset();

        assert_eq!(manager.attempts, 0);
        assert_eq!(manager.current_delay, Duration::from_secs(1));
    }
}
