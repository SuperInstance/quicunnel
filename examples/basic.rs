//! Basic tunnel example
//!
//! Demonstrates how to create a QUIC tunnel and send a request.

use quicunnel::{Tunnel, TunnelConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    // Initialize logging
    tracing_subscriber::fmt::init();

    // Configure tunnel
    let config = TunnelConfig {
        server_url: "https://quic.example.com:443".to_string(),
        client_id: "example-client".to_string(),
        cert_path: PathBuf::from("/path/to/cert.pem"),
        key_path: PathBuf::from("/path/to/key.pem"),
        ..Default::default()
    };

    // Create tunnel
    let mut tunnel = Tunnel::new(config)?;

    // Connect to server
    println!("Connecting to server...");
    tunnel.connect().await?;
    println!("Connected!");

    // Send a request
    let request = b"Hello, QUIC!";
    println!("Sending request: {} bytes", request.len());

    let response = tunnel.request(request).await?;
    println!("Received response: {} bytes", response.len());

    // Disconnect
    tunnel.disconnect().await?;
    println!("Disconnected");

    Ok(())
}
