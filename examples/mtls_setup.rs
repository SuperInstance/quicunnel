//! mTLS setup example
//!
//! Demonstrates how to generate certificates for mTLS authentication.

use quicunnel::tls::generate_device_certificate;
use std::fs::File;
use std::io::Write;
use std::path::PathBuf;

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let client_id = "my-client-123";

    // Generate certificate and key
    println!("Generating certificate for client: {}", client_id);
    let (cert, key) = generate_device_certificate(client_id)?;

    // Save certificate
    let cert_path = PathBuf::from(format!("{}.crt", client_id));
    let mut cert_file = File::create(&cert_path)?;
    cert_file.write_all(&cert.0)?;
    println!("Certificate saved to: {:?}", cert_path);

    // Save private key
    let key_path = PathBuf::from(format!("{}.key", client_id));
    let mut key_file = File::create(&key_path)?;
    key_file.write_all(&key.0)?;
    println!("Private key saved to: {:?}", key_path);

    // Convert to PEM format for use with QUIC tunnel
    println!("\nTo use with quicunnel:");
    println!("1. Convert cert to PEM: openssl x509 -in {}.crt -out {}.pem -outform PEM", client_id, client_id);
    println!("2. Convert key to PEM: openssl pkcs8 -topk8 -inform DER -in {}.key -out {}.pem -outform PEM -nocrypt", client_id, client_id);
    println!("3. Update TunnelConfig to use the .pem files");

    Ok(())
}
