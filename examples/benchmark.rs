//! Performance benchmark example
//!
//! Demonstrates measuring QUIC tunnel performance.

use quicunnel::{Tunnel, TunnelConfig};
use std::path::PathBuf;
use std::time::Instant;

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    tracing_subscriber::fmt::init();

    let config = TunnelConfig {
        server_url: "https://quic.example.com:443".to_string(),
        client_id: "benchmark-example".to_string(),
        cert_path: PathBuf::from("/path/to/cert.pem"),
        key_path: PathBuf::from("/path/to/key.pem"),
        ..Default::default()
    };

    let mut tunnel = Tunnel::new(config)?;

    // Measure connection time
    let start = Instant::now();
    tunnel.connect().await?;
    let connect_time = start.elapsed();
    println!("Connection time: {:?}", connect_time);

    // Benchmark different payload sizes
    let sizes = vec![1024, 10 * 1024, 100 * 1024, 1024 * 1024];

    for size in sizes {
        let data = vec![0u8; size];

        // Warmup
        let _ = tunnel.request(&data).await;

        // Measure
        let iterations = 10;
        let start = Instant::now();

        for _ in 0..iterations {
            let _ = tunnel.request(&data).await;
        }

        let elapsed = start.elapsed();
        let avg_time = elapsed / iterations;
        let throughput = (size as f64 * iterations as f64) / elapsed.as_secs_f64();

        println!(
            "Payload: {:>8} bytes | Avg: {:>8.2?} | Throughput: {:>10.2} MB/s",
            size,
            avg_time,
            throughput / (1024.0 * 1024.0)
        );
    }

    // Get final stats
    let stats = tunnel.stats().await;
    println!("\nTotal stats:");
    println!("  Bytes sent: {}", stats.total_bytes_sent);
    println!("  Bytes received: {}", stats.total_bytes_received);
    println!("  Requests: {}", stats.requests_sent);
    println!("  Success rate: {:.2}%", stats.success_rate() * 100.0);

    tunnel.disconnect().await?;

    Ok(())
}
