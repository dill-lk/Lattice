//! Error types for lattice-network

/// Network error type
#[derive(Debug, thiserror::Error)]
pub enum NetworkError {
    #[error("peer error: {0}")]
    Peer(String),

    #[error("connection error: {0}")]
    Connection(String),

    #[error("protocol error: {0}")]
    Protocol(String),

    #[error("sync error: {0}")]
    Sync(String),

    #[error("serialization error: {0}")]
    Serialization(String),

    #[error("timeout: {0}")]
    Timeout(String),

    #[error("invalid message: {0}")]
    InvalidMessage(String),

    #[error("channel error: {0}")]
    Channel(String),
}

/// Result type for network operations
pub type Result<T> = std::result::Result<T, NetworkError>;
