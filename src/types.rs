//! Tunnel types and configuration

use std::path::PathBuf;
use std::time::{Duration, Instant};

/// Configuration for QUIC tunnel connection
#[derive(Debug, Clone)]
pub struct TunnelConfig {
    /// Server endpoint URL (e.g., `https://quic.server.com:443`)
    pub server_url: String,

    /// Unique client identifier
    pub client_id: String,

    /// Path to client certificate (PEM format)
    pub cert_path: PathBuf,

    /// Path to client private key (PEM format)
    pub key_path: PathBuf,

    /// Interval between heartbeat messages
    pub heartbeat_interval: Duration,

    /// Delay before reconnection attempt
    pub reconnect_delay: Duration,

    /// Maximum reconnection attempts before giving up
    pub max_reconnect_attempts: u32,

    /// Connection timeout
    pub connect_timeout: Duration,

    /// Read timeout for responses
    pub read_timeout: Duration,

    /// Maximum response size (10 MB default)
    pub max_response_size: usize,
}

impl Default for TunnelConfig {
    fn default() -> Self {
        Self {
            server_url: String::new(),
            client_id: String::new(),
            cert_path: PathBuf::new(),
            key_path: PathBuf::new(),
            heartbeat_interval: Duration::from_secs(30),
            reconnect_delay: Duration::from_secs(5),
            max_reconnect_attempts: 10,
            connect_timeout: Duration::from_secs(30),
            read_timeout: Duration::from_secs(60),
            max_response_size: 10 * 1024 * 1024, // 10 MB
        }
    }
}

/// Current state of the tunnel connection
#[derive(Debug, Clone, PartialEq)]
pub enum TunnelState {
    /// Not connected
    Disconnected,

    /// Attempting to establish connection
    Connecting {
        /// Timestamp when connection attempt started
        since: Instant,
    },

    /// Connected and healthy
    Connected {
        /// Timestamp when connection was established
        since: Instant,
        /// Current latency to server endpoint in milliseconds
        latency_ms: u32,
    },

    /// Connection lost, attempting to reconnect
    Reconnecting {
        /// Current reconnection attempt number
        attempt: u32,
        /// Error message from last failed connection attempt
        last_error: String,
    },

    /// Connection failed permanently
    Failed {
        /// Error that caused permanent failure
        error: String,
        /// Timestamp when failure occurred
        at: Instant,
    },
}

impl TunnelState {
    /// Check if currently connected
    pub fn is_connected(&self) -> bool {
        matches!(self, TunnelState::Connected { .. })
    }

    /// Check if connection is healthy (connected and low latency)
    pub fn is_healthy(&self) -> bool {
        match self {
            TunnelState::Connected { latency_ms, .. } => *latency_ms < 500,
            _ => false,
        }
    }
}

/// Statistics for tunnel connection
#[derive(Debug, Clone, Default)]
pub struct TunnelStats {
    /// Total bytes sent to server endpoint
    pub total_bytes_sent: u64,
    /// Total bytes received from server endpoint
    pub total_bytes_received: u64,
    /// Total heartbeat messages sent
    pub heartbeats_sent: u64,
    /// Total heartbeat acknowledgments received
    pub heartbeats_acked: u64,
    /// Total requests sent
    pub requests_sent: u64,
    /// Total successful requests
    pub requests_succeeded: u64,
    /// Total failed requests
    pub requests_failed: u64,
    /// Number of reconnection attempts
    pub reconnections: u32,
    /// Average latency in milliseconds
    pub avg_latency_ms: u32,
}

impl TunnelStats {
    /// Calculate success rate
    pub fn success_rate(&self) -> f64 {
        if self.requests_sent == 0 {
            return 1.0;
        }
        self.requests_succeeded as f64 / self.requests_sent as f64
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_tunnel_config_defaults() {
        let config = TunnelConfig::default();
        assert_eq!(config.heartbeat_interval, Duration::from_secs(30));
        assert_eq!(config.max_reconnect_attempts, 10);
        assert_eq!(config.connect_timeout, Duration::from_secs(30));
        assert_eq!(config.max_response_size, 10 * 1024 * 1024);
    }

    #[test]
    fn test_tunnel_state_transitions() {
        // Disconnected -> Connecting
        let mut state = TunnelState::Connecting {
            since: Instant::now(),
        };
        assert!(matches!(state, TunnelState::Connecting { .. }));

        // Connecting -> Connected
        state = TunnelState::Connected {
            since: Instant::now(),
            latency_ms: 50,
        };
        assert!(matches!(state, TunnelState::Connected { .. }));
        assert!(state.is_connected());
        assert!(state.is_healthy());
    }

    #[test]
    fn test_tunnel_stats_accumulation() {
        let mut stats = TunnelStats::default();

        stats.total_bytes_sent += 1024;
        stats.heartbeats_sent += 1;
        stats.requests_sent += 1;
        stats.requests_succeeded += 1;

        assert_eq!(stats.total_bytes_sent, 1024);
        assert_eq!(stats.heartbeats_sent, 1);
        assert_eq!(stats.success_rate(), 1.0);
    }
}
