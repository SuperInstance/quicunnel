//! Heartbeat service for keeping tunnel alive

use crate::error::Result;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::Duration;
use tokio::sync::broadcast;
use tokio::sync::RwLock;

/// Heartbeat service configuration
#[derive(Debug, Clone)]
pub struct HeartbeatConfig {
    /// Interval between heartbeat messages
    pub interval: Duration,
    /// Timeout for heartbeat acknowledgments
    pub timeout: Duration,
    /// Client identifier
    pub client_id: String,
}

impl Default for HeartbeatConfig {
    fn default() -> Self {
        Self {
            interval: Duration::from_secs(30),
            timeout: Duration::from_secs(10),
            client_id: "client-unknown".to_string(),
        }
    }
}

/// Heartbeat service
///
/// Sends periodic heartbeats to keep the tunnel alive and detect failures.
pub struct HeartbeatService {
    config: HeartbeatConfig,
    sequence: Arc<AtomicU64>,
    connection: Arc<RwLock<Option<quinn::Connection>>>,
    shutdown: broadcast::Sender<()>,
}

impl HeartbeatService {
    /// Create a new heartbeat service
    pub fn new(config: HeartbeatConfig) -> Self {
        let (shutdown, _) = broadcast::channel(1);
        Self {
            config,
            sequence: Arc::new(AtomicU64::new(0)),
            connection: Arc::new(RwLock::new(None)),
            shutdown,
        }
    }

    /// Set the active connection
    pub async fn set_connection(&self, conn: quinn::Connection) {
        let mut connection = self.connection.write().await;
        *connection = Some(conn);
    }

    /// Clear the connection (on disconnect)
    pub async fn clear_connection(&self) {
        let mut connection = self.connection.write().await;
        *connection = None;
    }

    /// Spawn the heartbeat task
    ///
    /// Returns a JoinHandle that can be used to wait for shutdown
    pub fn spawn(&self) -> tokio::task::JoinHandle<()> {
        let interval = self.config.interval;
        let sequence = self.sequence.clone();
        let connection_lock = self.connection.clone();
        let client_id = self.config.client_id.clone();
        let mut shutdown_rx = self.shutdown.subscribe();

        tokio::spawn(async move {
            let mut interval_timer = tokio::time::interval(interval);

            loop {
                tokio::select! {
                    _ = interval_timer.tick() => {
                        let conn_opt = connection_lock.read().await;
                        if let Some(conn) = conn_opt.as_ref() {
                            let seq = sequence.fetch_add(1, Ordering::SeqCst);

                            // Send heartbeat
                            if let Err(e) = Self::send_heartbeat(conn, seq, &client_id).await {
                                tracing::warn!("Heartbeat failed: {}", e);
                            }
                        }
                    }
                    _ = shutdown_rx.recv() => {
                        tracing::info!("Heartbeat service shutting down");
                        break;
                    }
                }
            }
        })
    }

    /// Send a single heartbeat
    async fn send_heartbeat(
        conn: &quinn::Connection,
        sequence: u64,
        client_id: &str,
    ) -> Result<()> {
        // Create heartbeat message
        let heartbeat = serde_json::json!({
            "type": "heartbeat",
            "client_id": client_id,
            "timestamp": chrono::Utc::now().timestamp_millis(),
            "sequence": sequence,
        });

        let data = serde_json::to_vec(&heartbeat)
            .map_err(|e| crate::error::QuicunnelError::Serialization(e.to_string()))?;

        // Send on unidirectional stream
        let mut send = conn.open_uni().await
            .map_err(|e| crate::error::QuicunnelError::tunnel_connection(format!("Failed to open stream: {}", e)))?;

        // Message type: Heartbeat (0x01)
        send.write_all(&[0x01]).await
            .map_err(|e| crate::error::QuicunnelError::tunnel_connection(format!("Failed to write type: {}", e)))?;

        // Length (4 bytes big-endian)
        let len = data.len() as u32;
        send.write_all(&len.to_be_bytes()).await
            .map_err(|e| crate::error::QuicunnelError::tunnel_connection(format!("Failed to write length: {}", e)))?;

        // Payload
        send.write_all(&data).await
            .map_err(|e| crate::error::QuicunnelError::tunnel_connection(format!("Failed to write payload: {}", e)))?;

        send.finish().await
            .map_err(|e| crate::error::QuicunnelError::tunnel_connection(format!("Failed to finish stream: {}", e)))?;

        tracing::trace!("Heartbeat sent: seq={}", sequence);

        Ok(())
    }

    /// Shutdown the heartbeat service
    pub fn shutdown(&self) {
        let _ = self.shutdown.send(());
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_heartbeat_config_defaults() {
        let config = HeartbeatConfig::default();
        assert_eq!(config.interval, Duration::from_secs(30));
        assert_eq!(config.timeout, Duration::from_secs(10));
    }

    #[test]
    fn test_heartbeat_service_creation() {
        let service = HeartbeatService::new(HeartbeatConfig::default());
        assert_eq!(service.sequence.load(Ordering::SeqCst), 0);
    }
}
