//! Wallet error types

use thiserror::Error;

/// Wallet error type
#[derive(Debug, Error)]
pub enum WalletError {
    #[error("keystore error: {0}")]
    Keystore(String),

    #[error("invalid password")]
    InvalidPassword,

    #[error("encryption error: {0}")]
    Encryption(String),

    #[error("decryption error: {0}")]
    Decryption(String),

    #[error("invalid keystore format: {0}")]
    InvalidKeystoreFormat(String),

    #[error("io error: {0}")]
    Io(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("crypto error: {0}")]
    Crypto(String),

    #[error("transaction error: {0}")]
    Transaction(String),

    #[error("insufficient balance")]
    InsufficientBalance,

    #[error("invalid nonce")]
    InvalidNonce,

    #[error("missing field: {0}")]
    MissingField(String),
}

pub type Result<T> = std::result::Result<T, WalletError>;
