//! Error types for quicunnel

/// Main error type for quicunnel
#[derive(Debug, thiserror::Error)]
pub enum QuicunnelError {
    /// Certificate-related errors
    #[error("Certificate error: {0}")]
    Certificate(String),

    /// TLS configuration errors
    #[error("TLS error: {0}")]
    Tls(String),

    /// Tunnel connection errors
    #[error("Tunnel connection error: {0}")]
    TunnelConnection(String),

    /// Validation errors
    #[error("Validation error: {0}")]
    Validation(String),

    /// Serialization errors
    #[error("Serialization error: {0}")]
    Serialization(String),

    /// IO errors
    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    /// Other errors
    #[error("Error: {0}")]
    Other(String),
}

/// Type alias for Result with QuicunnelError
pub type Result<T> = std::result::Result<T, QuicunnelError>;

impl QuicunnelError {
    /// Create a certificate error
    pub fn certificate(msg: impl Into<String>) -> Self {
        Self::Certificate(msg.into())
    }

    /// Create a TLS error
    pub fn tls(msg: impl Into<String>) -> Self {
        Self::Tls(msg.into())
    }

    /// Create a tunnel connection error
    pub fn tunnel_connection(msg: impl Into<String>) -> Self {
        Self::TunnelConnection(msg.into())
    }

    /// Create a validation error
    pub fn validation(msg: impl Into<String>) -> Self {
        Self::Validation(msg.into())
    }

    /// Create a serialization error
    pub fn serialization(msg: impl Into<String>) -> Self {
        Self::Serialization(msg.into())
    }

    /// Create a generic error
    pub fn other(msg: impl Into<String>) -> Self {
        Self::Other(msg.into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_error_creation() {
        let err = QuicunnelError::certificate("test error");
        assert!(matches!(err, QuicunnelError::Certificate(_)));
        assert_eq!(err.to_string(), "Certificate error: test error");
    }

    #[test]
    fn test_error_display() {
        let err = QuicunnelError::tls("TLS failed");
        assert_eq!(err.to_string(), "TLS error: TLS failed");
    }
}
