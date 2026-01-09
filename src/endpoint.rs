//! QUIC endpoint for client connections

use crate::error::{QuicunnelError, Result};
use crate::tls::create_tls_config;
use quinn::{Endpoint, Connection};
use std::net::SocketAddr;
use std::path::Path;
use std::sync::Arc;
use std::time::Duration;

/// Create QUIC endpoint for client connections
///
/// # Arguments
/// * `cert_path` - Path to client certificate
/// * `key_path` - Path to client private key
///
/// # Returns
/// * Configured QUIC endpoint
///
/// # Example
///
/// ```rust,no_run
/// use quicunnel::endpoint::create_endpoint;
///
/// let endpoint = create_endpoint(
///     std::path::Path::new("/path/to/cert.pem"),
///     std::path::Path::new("/path/to/key.pem")
/// ).unwrap();
/// ```
pub fn create_endpoint(
    cert_path: &Path,
    key_path: &Path,
) -> Result<Endpoint> {
    // Create TLS config
    let tls_config = create_tls_config(cert_path, key_path)?;

    // Configure QUIC transport
    let mut transport = quinn::TransportConfig::default();
    transport.keep_alive_interval(Some(Duration::from_secs(10)));
    transport.max_idle_timeout(Some(Duration::from_secs(60).try_into()
        .map_err(|e| QuicunnelError::other(format!("Invalid duration: {}", e)))?));

    // Build QUIC client config
    let mut client_config = quinn::ClientConfig::new(tls_config);
    client_config.transport_config(Arc::new(transport));

    // Bind to random local port
    let bind_addr: SocketAddr = "0.0.0.0:0".parse()
        .map_err(|e| QuicunnelError::other(format!("Invalid bind address: {}", e)))?;
    let mut endpoint = Endpoint::client(bind_addr)
        .map_err(|e| QuicunnelError::tunnel_connection(format!("Failed to create endpoint: {}", e)))?;
    endpoint.set_default_client_config(client_config);

    Ok(endpoint)
}

/// Connect to server endpoint
///
/// # Arguments
/// * `endpoint` - QUIC endpoint
/// * `server_url` - Server URL
/// * `server_name` - Server name for TLS verification
///
/// # Returns
/// * Connected QUIC connection
///
/// # Example
///
/// ```rust,no_run
/// # use quicunnel::endpoint::{create_endpoint, connect_to_cloud};
/// # async fn example() -> Result<(), Box<dyn std::error::Error>> {
/// let endpoint = create_endpoint(
///     std::path::Path::new("/path/to/cert.pem"),
///     std::path::Path::new("/path/to/key.pem")
/// )?;
///
/// let conn = connect_to_cloud(
///     &endpoint,
///     "https://quic.server.com:443",
///     "quic.server.com"
/// ).await?;
/// # Ok(())
/// # }
/// ```
pub async fn connect_to_cloud(
    endpoint: &Endpoint,
    server_url: &str,
    server_name: &str,
) -> Result<Connection> {
    let addr = resolve_dns(server_url).await?;

    let connection = endpoint
        .connect(addr, server_name)
        .map_err(|e| QuicunnelError::tunnel_connection(format!("Failed to connect: {}", e)))?
        .await
        .map_err(|e| QuicunnelError::tunnel_connection(format!("Connection failed: {}", e)))?;

    tracing::info!(
        "Connected to server: addr={}, server_name={}",
        addr,
        server_name
    );

    Ok(connection)
}

/// Resolve DNS hostname to SocketAddr
///
/// # Arguments
/// * `url` - Server URL (e.g., `https://quic.server.com:443`)
///
/// # Returns
/// * Resolved SocketAddr
async fn resolve_dns(url: &str) -> Result<SocketAddr> {
    let parsed = url::Url::parse(url)
        .map_err(|e| QuicunnelError::tunnel_connection(format!("Invalid URL: {}", e)))?;

    let host = parsed.host_str()
        .ok_or_else(|| QuicunnelError::tunnel_connection("No host in URL"))?;

    let port = parsed.port().unwrap_or(443);

    // Resolve DNS
    let mut addrs = tokio::net::lookup_host(format!("{}:{}", host, port))
        .await
        .map_err(|e| QuicunnelError::tunnel_connection(format!("DNS resolution failed: {}", e)))?;

    // Prefer IPv4 for now
    addrs
        .find(|addr| addr.is_ipv4())
        .or_else(|| addrs.next())
        .ok_or_else(|| QuicunnelError::tunnel_connection("No addresses found"))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_dns() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let result = runtime.block_on(async {
            resolve_dns("https://example.com:443").await
        });

        // Should resolve example.com
        assert!(result.is_ok());
        let addr = result.unwrap();
        assert!(addr.port() == 443 || addr.is_ipv4());
    }

    #[test]
    fn test_invalid_url() {
        let runtime = tokio::runtime::Runtime::new().unwrap();

        let result = runtime.block_on(async {
            resolve_dns("not-a-url").await
        });

        assert!(result.is_err());
    }
}
