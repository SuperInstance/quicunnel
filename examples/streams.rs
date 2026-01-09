//! Stream multiplexing example
//!
//! Demonstrates opening multiple concurrent streams over a single QUIC connection.

use quicunnel::{Tunnel, TunnelConfig};
use std::path::PathBuf;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = TunnelConfig {
        server_url: "https://quic.example.com:443".to_string(),
        client_id: "streams-example".to_string(),
        cert_path: PathBuf::from("/path/to/cert.pem"),
        key_path: PathBuf::from("/path/to/key.pem"),
        ..Default::default()
    };

    let mut tunnel = Tunnel::new(config)?;
    tunnel.connect().await?;

    // Open multiple concurrent streams
    let mut tasks = Vec::new();

    for i in 0..5 {
        let request = format!("Stream {}", i);
        let data = request.into_bytes();

        // Spawn task for each stream
        let task = tokio::spawn(async move {
            // Simulate sending on a stream
            println!("Sending on stream {}: {} bytes", i, data.len());
            tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            println!("Stream {} complete", i);
            Ok::<(), Box<dyn std::error::Error>>(())
        });

        tasks.push(task);
    }

    // Wait for all streams to complete
    for task in tasks {
        task.await??;
    }

    println!("All streams completed");

    tunnel.disconnect().await?;
    Ok(())
}
