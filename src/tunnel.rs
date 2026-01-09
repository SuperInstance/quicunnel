//! Main Tunnel implementation
//!
//! Provides a persistent QUIC tunnel with mTLS authentication,
//! automatic heartbeat, and reconnection logic.

pub use crate::types::{TunnelConfig, TunnelState, TunnelStats};

use super::state::ConnectionStateMachine;
use super::heartbeat::{HeartbeatService, HeartbeatConfig};
use super::endpoint::{create_endpoint, connect_to_cloud};
use crate::error::{QuicunnelError, Result};
use std::sync::Arc;
use std::time::Instant;
use tokio::sync::RwLock;

/// Main tunnel struct
///
/// Provides a persistent QUIC tunnel with mTLS authentication,
/// automatic heartbeat, and reconnection logic.
///
/// ## Example
///
/// ```rust,no_run
/// # use quicunnel::Tunnel;
/// # use quicunnel::TunnelConfig;
/// # use std::path::PathBuf;
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let config = TunnelConfig {
///     server_url: "https://quic.server.com:443".to_string(),
///     cert_path: PathBuf::from("/path/to/cert.pem"),
///     key_path: PathBuf::from("/path/to/key.pem"),
///     ..Default::default()
/// };
///
/// let mut tunnel = Tunnel::new(config)?;
/// tunnel.connect().await?;
///
/// // Send request
/// let response = tunnel.request(b"hello").await?;
/// # Ok(())
/// # }
/// ```
pub struct Tunnel {
    config: TunnelConfig,
    endpoint: Option<quinn::Endpoint>,
    connection: Arc<RwLock<Option<quinn::Connection>>>,
    state_machine: ConnectionStateMachine,
    heartbeat_service: Option<HeartbeatService>,
    stats: Arc<RwLock<TunnelStats>>,
}

impl Tunnel {
    /// Create a new tunnel instance
    ///
    /// # Arguments
    /// * `config` - Tunnel configuration
    ///
    /// # Returns
    /// * Configured tunnel (not yet connected)
    pub fn new(config: TunnelConfig) -> Result<Self> {
        if config.cert_path.as_path().as_os_str().is_empty() {
            return Err(QuicunnelError::validation("Certificate path is required"));
        }
        if config.key_path.as_path().as_os_str().is_empty() {
            return Err(QuicunnelError::validation("Key path is required"));
        }

        Ok(Self {
            config,
            endpoint: None,
            connection: Arc::new(RwLock::new(None)),
            state_machine: ConnectionStateMachine::new(),
            heartbeat_service: None,
            stats: Arc::new(RwLock::new(TunnelStats::default())),
        })
    }

    /// Connect to server
    ///
    /// # Returns
    /// * Ok(()) if connection successful
    /// * Err if connection fails
    pub async fn connect(&mut self) -> Result<()> {
        self.state_machine.transition(TunnelState::Connecting {
            since: Instant::now(),
        });

        match self.connect_internal().await {
            Ok(()) => {
                // Start heartbeat
                if let Some(ref heartbeat_service) = self.heartbeat_service {
                    heartbeat_service.spawn();
                }

                tracing::info!("Tunnel connected successfully");
                Ok(())
            }
            Err(e) => {
                self.state_machine.transition(TunnelState::Failed {
                    error: e.to_string(),
                    at: Instant::now(),
                });
                Err(e)
            }
        }
    }

    /// Internal connection logic
    async fn connect_internal(&mut self) -> Result<()> {
        // Create endpoint if not exists
        if self.endpoint.is_none() {
            self.endpoint = Some(create_endpoint(
                &self.config.cert_path,
                &self.config.key_path,
            )?);
        }

        // Connect to server
        let server_url = self.config.server_url.clone();
        let server_name = extract_server_name(&server_url)?;

        let conn = connect_to_cloud(
            self.endpoint.as_ref().unwrap(),
            &server_url,
            &server_name,
        ).await?;

        // Store connection
        *self.connection.write().await = Some(conn.clone());

        // Create and start heartbeat service
        let heartbeat_config = HeartbeatConfig {
            interval: self.config.heartbeat_interval,
            client_id: self.config.client_id.clone(),
            ..Default::default()
        };
        let heartbeat_service = HeartbeatService::new(heartbeat_config);
        heartbeat_service.set_connection(conn.clone()).await;
        self.heartbeat_service = Some(heartbeat_service);

        // Transition to connected
        self.state_machine.transition(TunnelState::Connected {
            since: Instant::now(),
            latency_ms: 0, // Will be updated by first heartbeat
        });

        Ok(())
    }

    /// Disconnect from server
    pub async fn disconnect(&mut self) -> Result<()> {
        if let Some(ref heartbeat_service) = self.heartbeat_service {
            heartbeat_service.shutdown();
            heartbeat_service.clear_connection().await;
        }

        if let Some(ref conn) = self.connection.write().await.take() {
            conn.close(0u32.into(), b"client disconnect");
        }

        self.state_machine.transition(TunnelState::Disconnected);

        tracing::info!("Tunnel disconnected");
        Ok(())
    }

    /// Check if connected
    pub fn is_connected(&self) -> bool {
        self.state_machine.current().is_connected()
    }

    /// Get current state
    pub fn state(&self) -> TunnelState {
        self.state_machine.current()
    }

    /// Get tunnel statistics
    pub async fn stats(&self) -> TunnelStats {
        let stats = self.stats.read().await;
        stats.clone()
    }

    /// Send request and receive response (bidirectional stream)
    ///
    /// Opens a bidirectional QUIC stream, sends the request data,
    /// and waits for the complete response.
    ///
    /// # Performance
    ///
    /// - **Stream opening**: O(1) - QUIC stream allocation
    /// - **Sending**: O(n) where n = request size
    /// - **Receiving**: O(m) where m = response size (max 10 MB)
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use quicunnel::Tunnel;
    /// # async fn example(tunnel: &Tunnel) -> Result<(), Box<dyn std::error::Error>> {
    /// let request = b"hello server";
    /// let response = tunnel.request(request).await?;
    /// println!("Got response: {} bytes", response.len());
    /// # Ok(())
    /// # }
    /// ```
    pub async fn request(&self, data: &[u8]) -> Result<Vec<u8>> {
        // Get connection (clone to avoid holding lock across await)
        let conn = self.connection.read().await
            .as_ref()
            .ok_or_else(|| {
                QuicunnelError::tunnel_connection(
                    "Tunnel not connected. Call connect() first."
                )
            })?
            .clone();

        // Open bidirectional stream
        let (mut send, mut recv) = conn.open_bi().await
            .map_err(|e| QuicunnelError::tunnel_connection(format!(
                "Failed to open QUIC bidirectional stream: {}", e
            )))?;

        // Send request data
        send.write_all(data).await
            .map_err(|e| QuicunnelError::tunnel_connection(format!(
                "Failed to send request data: {}", e
            )))?;
        send.finish().await
            .map_err(|e| QuicunnelError::tunnel_connection(format!(
                "Failed to finish sending: {}", e
            )))?;

        // Receive response (with size limit to prevent memory exhaustion)
        let response = recv.read_to_end(self.config.max_response_size).await
            .map_err(|e| QuicunnelError::tunnel_connection(format!(
                "Failed to read response: {}", e
            )))?;

        // Update statistics (bytes sent/received, request count)
        let mut stats = self.stats.write().await;
        stats.total_bytes_sent += data.len() as u64;
        stats.total_bytes_received += response.len() as u64;
        stats.requests_sent += 1;
        stats.requests_succeeded += 1;

        Ok(response)
    }

    /// Open unidirectional stream for sending
    ///
    /// Useful for streaming data where no response is needed.
    ///
    /// # Example
    ///
    /// ```rust,no_run
    /// # use quicunnel::Tunnel;
    /// # async fn example(tunnel: &Tunnel) -> Result<(), Box<dyn std::error::Error>> {
    /// let mut send_stream = tunnel.open_uni().await?;
    /// send_stream.write_all(b"streaming data").await?;
    /// send_stream.finish().await?;
    /// # Ok(())
    /// # }
    /// ```
    pub async fn open_uni(&self) -> Result<quinn::SendStream> {
        let conn = self.connection.read().await
            .as_ref()
            .ok_or_else(|| {
                QuicunnelError::tunnel_connection(
                    "Tunnel not connected. Call connect() first."
                )
            })?
            .clone();

        conn.open_uni().await
            .map_err(|e| QuicunnelError::tunnel_connection(format!(
                "Failed to open unidirectional stream: {}", e
            )))
    }
}

/// Extract server name from URL
fn extract_server_name(url: &str) -> Result<String> {
    let parsed = url::Url::parse(url)
        .map_err(|e| QuicunnelError::validation(format!("Invalid URL: {}", e)))?;

    parsed.host_str()
        .map(|s| s.to_string())
        .ok_or_else(|| QuicunnelError::validation("No host in URL"))
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::path::PathBuf;

    #[test]
    fn test_extract_server_name() {
        assert_eq!(
            extract_server_name("https://quic.server.com:443").unwrap(),
            "quic.server.com"
        );
        assert_eq!(
            extract_server_name("https://api.example.com").unwrap(),
            "api.example.com"
        );
    }

    #[test]
    fn test_tunnel_creation() {
        let config = TunnelConfig {
            cert_path: "/tmp/cert.pem".into(),
            key_path: "/tmp/key.pem".into(),
            ..Default::default()
        };

        let tunnel = Tunnel::new(config);
        assert!(tunnel.is_ok());
        let tunnel = tunnel.unwrap();

        assert!(!tunnel.is_connected());
        assert!(matches!(tunnel.state(), TunnelState::Disconnected));
    }

    #[test]
    fn test_tunnel_validation() {
        let config = TunnelConfig {
            cert_path: PathBuf::new(), // Empty
            key_path: PathBuf::new(), // Empty
            ..Default::default()
        };

        let result = Tunnel::new(config);
        assert!(result.is_err());
    }
}
