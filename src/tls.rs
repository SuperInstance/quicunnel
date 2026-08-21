//! TLS configuration for QUIC tunnel
//!
//! This module provides TLS 1.3 configuration with mTLS support.

use crate::error::{QuicunnelError, Result};
use rustls::pki_types::{CertificateDer, PrivateKeyDer};
use rustls::{ClientConfig, RootCertStore};
use rustls_pemfile::{certs, ec_private_keys, pkcs8_private_keys, rsa_private_keys};
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
pub fn create_tls_config(cert_path: &Path, key_path: &Path) -> Result<Arc<ClientConfig>> {
    // Load client certificate
    let cert_file = File::open(cert_path).map_err(|e| {
        QuicunnelError::certificate(format!("Failed to open certificate file: {}", e))
    })?;
    let mut cert_reader = BufReader::new(cert_file);
    let cert_vec = certs(&mut cert_reader)
        .map_err(|e| QuicunnelError::certificate(format!("Failed to parse certificate: {}", e)))?;

    if cert_vec.is_empty() {
        return Err(QuicunnelError::certificate("No certificates found in file"));
    }

    let client_certs: Vec<CertificateDer<'static>> =
        cert_vec.into_iter().map(CertificateDer::from).collect();

    // Load client private key. PEM markers determine the key encoding, so try
    // each parser in turn (RSA PKCS#1, EC SEC1, then PKCS#8).
    let key = load_private_key(key_path)?;

    // Build root certificate store from the Mozilla/webpki bundled roots.
    let mut roots = RootCertStore::empty();
    roots.extend(webpki_roots::TLS_SERVER_ROOTS.iter().cloned());

    // Build client config with mTLS
    let config = ClientConfig::builder()
        .with_root_certificates(roots)
        .with_client_auth_cert(client_certs, key)
        .map_err(|e| QuicunnelError::tls(format!("Failed to build client config: {}", e)))?;

    Ok(Arc::new(config))
}

/// Read the first PEM private key from `key_path`, trying RSA, EC and PKCS#8.
fn load_private_key(key_path: &Path) -> Result<PrivateKeyDer<'static>> {
    let raw = read_first_key(key_path)?;
    // `PrivateKeyDer::try_from` inspects the DER encoding to classify the key,
    // so we can hand it the raw bytes regardless of which PEM section produced them.
    PrivateKeyDer::try_from(raw)
        .map_err(|e| QuicunnelError::certificate(format!("Failed to decode private key: {}", e)))
}

fn read_first_key(key_path: &Path) -> Result<Vec<u8>> {
    let try_parse = |parse: fn(&mut dyn std::io::BufRead) -> std::io::Result<Vec<Vec<u8>>>| -> Result<Option<Vec<u8>>> {
        let mut reader = BufReader::new(
            File::open(key_path)
                .map_err(|e| QuicunnelError::certificate(format!("Failed to open key file: {}", e)))?,
        );
        let keys = parse(&mut reader)
            .map_err(|e| QuicunnelError::certificate(format!("Failed to parse key: {}", e)))?;
        Ok(keys.into_iter().next())
    };

    if let Some(k) = try_parse(rsa_private_keys)? {
        return Ok(k);
    }
    if let Some(k) = try_parse(ec_private_keys)? {
        return Ok(k);
    }
    if let Some(k) = try_parse(pkcs8_private_keys)? {
        return Ok(k);
    }

    Err(QuicunnelError::certificate("No private key found in file"))
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
/// * `(CertificateDer, PrivateKeyDer)` pair (DER-encoded) for the client
///
/// # Example
///
/// ```rust,no_run
/// use quicunnel::tls::generate_device_certificate;
///
/// let (cert, key) = generate_device_certificate("client-123").unwrap();
/// // `cert` / `key` deref to `&[u8]` (raw DER) and can be written to disk
/// // or fed directly into rustls.
/// ```
pub fn generate_device_certificate(
    client_id: &str,
) -> Result<(CertificateDer<'static>, PrivateKeyDer<'static>)> {
    use rcgen::{CertificateParams, DnType, KeyPair, SanType};

    // Generate key pair (defaults to PKCS_ECDSA_P256_SHA256)
    let key_pair = KeyPair::generate()
        .map_err(|e| QuicunnelError::certificate(format!("Failed to generate key pair: {}", e)))?;

    // Build certificate parameters
    let mut params = CertificateParams::default();
    params
        .distinguished_name
        .push(DnType::CommonName, format!("client-{}", client_id));
    params
        .distinguished_name
        .push(DnType::OrganizationName, "Quicunnel");
    params.not_before = time::OffsetDateTime::now_utc();
    params.not_after = time::OffsetDateTime::now_utc() + time::Duration::days(365);

    // Subject alternative name (DNS name for client)
    params.subject_alt_names = vec![SanType::DnsName(
        format!("{}.client.quicunnel.local", client_id)
            .try_into()
            .map_err(|e| QuicunnelError::certificate(format!("Invalid subject alt name: {}", e)))?,
    )];

    // Extended key usage for client auth
    params.extended_key_usages = vec![rcgen::ExtendedKeyUsagePurpose::ClientAuth];

    let cert = params.self_signed(&key_pair).map_err(|e| {
        QuicunnelError::certificate(format!("Failed to generate certificate: {}", e))
    })?;

    let cert_der = cert.der().clone();
    let key = PrivateKeyDer::Pkcs8(key_pair.serialize_der().into());

    Ok((cert_der, key))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_device_certificate() {
        let client_id = "test-client-123";
        let (cert, key) = generate_device_certificate(client_id).unwrap();

        // Verify we got a non-empty certificate and key (DER bytes).
        assert!(!cert.as_ref().is_empty());
        assert!(!key.secret_der().is_empty());
    }

    #[test]
    fn test_missing_cert_file() {
        let result = create_tls_config(
            Path::new("/nonexistent/cert.pem"),
            Path::new("/nonexistent/key.pem"),
        );

        assert!(result.is_err());
        match result.unwrap_err() {
            QuicunnelError::Certificate(_) => {}
            _ => panic!("Expected Certificate error"),
        }
    }

    #[test]
    fn test_distinct_clients_get_distinct_certs() {
        // The client_id is embedded in the certificate (CN + SAN DNS name), so
        // distinct ids must yield distinct DER encodings.
        let (cert_a, _) = generate_device_certificate("client-a").unwrap();
        let (cert_b, _) = generate_device_certificate("client-b").unwrap();
        assert_ne!(cert_a.as_ref(), cert_b.as_ref());

        // Re-generating the same id twice still produces different output because
        // a fresh key pair is minted each time (sanity check that we are not
        // accidentally returning cached/static material).
        let (c1, k1) = generate_device_certificate("same-id").unwrap();
        let (c2, k2) = generate_device_certificate("same-id").unwrap();
        assert_ne!(k1.secret_der(), k2.secret_der());
        // The certs differ too (different key -> different SPKI).
        assert_ne!(c1.as_ref(), c2.as_ref());
    }
}
