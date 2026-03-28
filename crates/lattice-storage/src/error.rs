//! Storage error types

use thiserror::Error;

/// Storage operation errors
#[derive(Debug, Error)]
pub enum StorageError {
    /// RocksDB error
    #[error("database error: {0}")]
    Database(#[from] rocksdb::Error),

    /// Serialization error
    #[error("serialization error: {0}")]
    Serialization(String),

    /// Deserialization error
    #[error("deserialization error: {0}")]
    Deserialization(String),

    /// Block not found
    #[error("block not found: {0}")]
    BlockNotFound(String),

    /// Transaction not found
    #[error("transaction not found: {0}")]
    TransactionNotFound(String),

    /// Account not found
    #[error("account not found")]
    AccountNotFound,

    /// Invalid block height
    #[error("invalid block height: {0}")]
    InvalidHeight(u64),

    /// Snapshot not found
    #[error("snapshot not found: {0}")]
    SnapshotNotFound(u64),

    /// Mempool full
    #[error("mempool is full")]
    MempoolFull,

    /// Duplicate transaction
    #[error("duplicate transaction")]
    DuplicateTransaction,

    /// Invalid state root
    #[error("invalid state root")]
    InvalidStateRoot,
}

/// Result type for storage operations
pub type Result<T> = std::result::Result<T, StorageError>;
