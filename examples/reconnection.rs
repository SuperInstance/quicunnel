//! Reconnection example
//!
//! Demonstrates automatic reconnection with exponential backoff.

use quicunnel::{Tunnel, TunnelConfig};
use std::path::PathBuf;
use std::time::Duration;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    // Configure with aggressive reconnection for demo
    let config = TunnelConfig {
        server_url: "https://quic.example.com:443".to_string(),
        client_id: "reconnect-example".to_string(),
        cert_path: PathBuf::from("/path/to/cert.pem"),
        key_path: PathBuf::from("/path/to/key.pem"),
        reconnect_delay: Duration::from_secs(2),
        max_reconnect_attempts: 5,
        ..Default::default()
    };

    let mut tunnel = Tunnel::new(config)?;

    // Try to connect (will fail if server is unavailable)
    match tunnel.connect().await {
        Ok(_) => {
            println!("Connected successfully!");
            println!("State: {:?}", tunnel.state());

            // Get stats
            let stats = tunnel.stats().await;
            println!("Stats: {}", stats.success_rate());

            tunnel.disconnect().await?;
        }
        Err(e) => {
            println!("Connection failed: {}", e);
            println!("State: {:?}", tunnel.state());
            println!("In production, reconnection would happen automatically");
        }
    }

    Ok(())
}
