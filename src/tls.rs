//! TLS configuration for QUIC tunnel
//!
//! This module provides TLS 1.3 configuration with mTLS support.

use crate::error::{QuicunnelError, Result};
use rustls::{Certificate, PrivateKey, ClientConfig, RootCertStore};
use rustls_pemfile::{certs, rsa_private_keys, ec_private_keys, pkcs8_private_keys};
use std::fs::File;
use std::io::BufReader;
use std::path::Path;
use std::sync::Arc;

/// Create TLS configuration for mTLS connection
///
/// # Arguments
/// * `cert_path` - Path to client certificate (PEM format)
/// * `key_path` - Path to client private key (PEM format)
///
/// # Returns
/// * `ClientConfig` configured for mTLS with system root CAs
///
/// # Example
///
/// ```rust,no_run
/// use quicunnel::tls::create_tls_config;
/// use std::path::Path;
///
/// let config = create_tls_config(
///     Path::new("/path/to/cert.pem"),
///     Path::new("/path/to/key.pem")
/// ).unwrap();
/// ```
pub fn create_tls_config(
    cert_path: &Path,
    key_path: &Path,
) -> Result<Arc<ClientConfig>> {
    // Load client certificate
    let cert_file = File::open(cert_path).map_err(|e| {
        QuicunnelError::certificate(format!("Failed to open certificate file: {}", e))
    })?;
    let mut cert_reader = BufReader::new(cert_file);
    let cert_vec = certs(&mut cert_reader).map_err(|e| {
        QuicunnelError::certificate(format!("Failed to parse certificate: {}", e))
    })?;
    let certs = cert_vec.into_iter().map(Certificate).collect::<Vec<_>>();

    if certs.is_empty() {
        return Err(QuicunnelError::certificate("No certificates found in file"));
    }

    // Load client private key (try RSA, then EC, then PKCS8)
    let key_file = File::open(key_path).map_err(|e| {
        QuicunnelError::certificate(format!("Failed to open key file: {}", e))
    })?;
    let mut key_reader = BufReader::new(key_file);

    // Try RSA private keys first
    let keys = rsa_private_keys(&mut key_reader).map_err(|e| {
        QuicunnelError::certificate(format!("Failed to parse RSA key: {}", e))
    })?;

    let key = if !keys.is_empty() {
        keys[0].clone()
    } else {
        // Reset reader and try EC keys
        key_reader = BufReader::new(File::open(key_path).map_err(|e| {
            QuicunnelError::certificate(format!("Failed to reopen key file: {}", e))
        })?);
        let keys = ec_private_keys(&mut key_reader).map_err(|e| {
            QuicunnelError::certificate(format!("Failed to parse EC key: {}", e))
        })?;

        if !keys.is_empty() {
            keys[0].clone()
        } else {
            // Reset reader and try PKCS8 keys
            key_reader = BufReader::new(File::open(key_path).map_err(|e| {
                QuicunnelError::certificate(format!("Failed to reopen key file: {}", e))
            })?);
            let keys = pkcs8_private_keys(&mut key_reader).map_err(|e| {
                QuicunnelError::certificate(format!("Failed to parse PKCS8 key: {}", e))
            })?;

            if !keys.is_empty() {
                keys[0].clone()
            } else {
                return Err(QuicunnelError::certificate("No private key found in file"));
            }
        }
    };

    let key = PrivateKey(key);

    // Build root certificate store with system CAs
    let mut roots = RootCertStore::empty();
    roots.add_trust_anchors(webpki_roots::TLS_SERVER_ROOTS.iter().map(|ta| {
        rustls::OwnedTrustAnchor::from_subject_spki_name_constraints(
            ta.subject,
            ta.spki,
            ta.name_constraints.as_ref().map(|nc| *nc),
        )
    }));

    // Build client config with mTLS
    let config = ClientConfig::builder()
        .with_safe_defaults()
        .with_root_certificates(roots)
        .with_client_auth_cert(certs, key)
        .map_err(|e| QuicunnelError::tls(format!("Failed to build client config: {}", e)))?;

    Ok(Arc::new(config))
}

/// Generate client certificate for testing
///
/// This function generates a self-signed certificate suitable for development
/// and testing. In production, certificates should be issued by a proper CA.
///
/// # Arguments
/// * `client_id` - Unique client identifier
///
/// # Returns
/// * `(Certificate, PrivateKey)` pair for the client
///
/// # Example
///
/// ```rust,no_run
/// use quicunnel::tls::generate_device_certificate;
///
/// let (cert, key) = generate_device_certificate("client-123").unwrap();
/// // Save cert and key to files for later use
/// ```
pub fn generate_device_certificate(
    client_id: &str,
) -> Result<(Certificate, PrivateKey)> {
    use rcgen::{Certificate as RcgenCert, CertificateParams, DnType, KeyPair, SanType};

    // Generate key pair
    let key_pair = KeyPair::generate(&rcgen::PKCS_ECDSA_P256_SHA256)
        .map_err(|e| QuicunnelError::certificate(format!("Failed to generate key pair: {}", e)))?;

    // Build certificate parameters
    let mut params = CertificateParams::default();
    params.distinguished_name.push(DnType::CommonName, format!("client-{}", client_id));
    params.distinguished_name.push(DnType::OrganizationName, "Quicunnel");
    params.not_before = time::OffsetDateTime::now_utc();
    params.not_after = time::OffsetDateTime::now_utc() + time::Duration::days(365);
    params.key_pair = Some(key_pair);

    // Subject alternative name (DNS name for client)
    params.subject_alt_names = vec![SanType::DnsName(format!("{}.client.quicunnel.local", client_id))];

    // Extended key usage for client auth
    params.extended_key_usages = vec![
        rcgen::ExtendedKeyUsagePurpose::ClientAuth,
    ];

    let cert = RcgenCert::from_params(params)
        .map_err(|e| QuicunnelError::certificate(format!("Failed to generate certificate: {}", e)))?;

    // Convert to rustls types
    let cert_der = cert.serialize_der()
        .map_err(|e| QuicunnelError::certificate(format!("Failed to serialize certificate: {}", e)))?;
    let key_der = cert.serialize_private_key_der();

    Ok((Certificate(cert_der), PrivateKey(key_der)))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_device_certificate() {
        let client_id = "test-client-123";
        let (cert, key) = generate_device_certificate(client_id).unwrap();

        // Verify we got a certificate and key
        assert!(!cert.0.is_empty());
        assert!(!key.0.is_empty());
    }

    #[test]
    fn test_missing_cert_file() {
        let result = create_tls_config(
            Path::new("/nonexistent/cert.pem"),
            Path::new("/nonexistent/key.pem"),
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            QuicunnelError::Certificate(_) => {},
            _ => panic!("Expected Certificate error"),
        }
    }
}
