//! # quicunnel
//!
//! A **client-side** QUIC tunnel library built on [quinn] and [rustls], with
//! mutual-TLS client authentication, a validated connection-state machine, and
//! a heartbeat sender. This is a library, not a server. See the crate README
//! for a full and honest status of what works today versus what is stubbed.
//!
//! ## What works
//!
//! - **mTLS client config** (`create_tls_config`) and **dev cert generation**
//!   (`generate_device_certificate`): rustls 0.23, webpki system roots.
//! - **Endpoint + connect** (`create_endpoint`, `connect_to_cloud`).
//! - **Request/response** over a bidirectional stream (`Tunnel::request`) and
//!   **fire-and-forget sends** (`Tunnel::open_uni`).
//! - **Connection state machine** with rejected illegal transitions
//!   (`ConnectionStateMachine`).
//! - **Heartbeat sender** that emits keep-alives on a uni stream
//!   (`HeartbeatService`).
//!
//! ## Not yet wired in
//!
//! - **Automatic reconnection**: `ReconnectManager` exists and is unit-tested,
//!   but `Tunnel` does not drive it; call `connect()` again yourself on drop.
//! - **Heartbeat ack/timeout**: `HeartbeatConfig::timeout` is accepted but
//!   unused; heartbeats are one-way.
//!
//! [quinn]: https://github.com/quinn-rs/quinn
//! [rustls]: https://github.com/rustls/rustls
//!
//! ## Quick Start
//!
//! ```rust,no_run
//! use quicunnel::{Tunnel, TunnelConfig};
//! use std::path::PathBuf;
//!
//! #[tokio::main]
//! async fn main() -> Result<(), Box<dyn std::error::Error>> {
//!     let config = TunnelConfig {
//!         server_url: "https://quic.server.com:443".to_string(),
//!         cert_path: PathBuf::from("/path/to/cert.pem"),
//!         key_path: PathBuf::from("/path/to/key.pem"),
//!         ..Default::default()
//!     };
//!
//!     let mut tunnel = Tunnel::new(config)?;
//!     tunnel.connect().await?;
//!
//!     // Send request
//!     let response = tunnel.request(b"Hello, QUIC!").await?;
//!     println!("Response: {} bytes", response.len());
//!
//!     Ok(())
//! }
//! ```
//!
//! ## License
//!
//! MIT OR Apache-2.0

pub mod endpoint;
pub mod error;
pub mod heartbeat;
pub mod reconnect;
pub mod state;
pub mod tls;
pub mod tunnel;
pub mod types;

pub use error::{QuicunnelError, Result};
pub use heartbeat::{HeartbeatConfig, HeartbeatService};
pub use reconnect::{ReconnectConfig, ReconnectManager};
pub use state::ConnectionStateMachine;
pub use tls::{create_tls_config, generate_device_certificate};
pub use tunnel::Tunnel;
pub use types::{TunnelConfig, TunnelState, TunnelStats};
