//! Error types for lattice-core

use crate::address::AddressError;
use crate::state::StateError;

/// Core error type
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("address error: {0}")]
    Address(#[from] AddressError),
    
    #[error("state error: {0}")]
    State(#[from] StateError),
    
    #[error("invalid block: {0}")]
    InvalidBlock(String),
    
    #[error("invalid transaction: {0}")]
    InvalidTransaction(String),
    
    #[error("serialization error: {0}")]
    Serialization(String),
}

/// Result type for core operations
pub type Result<T> = std::result::Result<T, CoreError>;
