//! Error types for the deploy crate.

/// Errors that can occur during deploy operations.
#[derive(Debug, thiserror::Error)]
pub enum DeployError {
    #[error("S3 operation failed: {message}")]
    S3 { message: String },

    #[error("Route53 operation failed: {message}")]
    Dns { message: String },

    #[error("AWS credential error: {message}")]
    Credential { message: String },

    #[error("Configuration error: {message}")]
    Config { message: String },

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),
}
