//! Error types for lattice-core

use crate::address::AddressError;
use crate::state::StateError;
use crate::{Amount, BlockHeight, Hash};

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

    #[error("serialization error")]
    SerializationError,

    #[error("serialization error: {0}")]
    Serialization(String),

    // Transaction errors
    #[error("transaction too large")]
    TransactionTooLarge,

    #[error("missing signature")]
    MissingSignature,

    #[error("missing public key")]
    MissingPublicKey,

    #[error("fee too low")]
    FeeTooLow,

    #[error("gas limit too low")]
    GasLimitTooLow,

    #[error("zero amount transfer")]
    ZeroAmountTransfer,

    #[error("invalid transaction data")]
    InvalidTransactionData,

    #[error("missing contract code")]
    MissingContractCode,

    #[error("invalid recipient")]
    InvalidRecipient,

    #[error("invalid signature")]
    InvalidSignature,

    #[error("invalid chain ID")]
    InvalidChainId,

    #[error("invalid nonce: expected {expected}, got {got}")]
    InvalidNonce { expected: u64, got: u64 },

    #[error("insufficient balance: required {required}, available {available}")]
    InsufficientBalance { required: Amount, available: Amount },

    #[error("amount overflow")]
    AmountOverflow,

    #[error("not a contract")]
    NotAContract,

    // Block errors
    #[error("block too large")]
    BlockTooLarge,

    #[error("too many transactions")]
    TooManyTransactions,

    #[error("invalid block version")]
    InvalidBlockVersion,

    #[error("zero difficulty")]
    ZeroDifficulty,

    #[error("invalid block height: expected {expected}, got {got}")]
    InvalidBlockHeight { expected: BlockHeight, got: BlockHeight },

    #[error("invalid previous hash")]
    InvalidPreviousHash,

    #[error("invalid timestamp")]
    InvalidTimestamp,

    #[error("block from future")]
    BlockFromFuture,

    #[error("invalid merkle root: expected {expected:?}, got {got:?}")]
    InvalidMerkleRoot { expected: Hash, got: Hash },

    #[error("duplicate transaction")]
    DuplicateTransaction,

    #[error("invalid genesis block")]
    InvalidGenesisBlock,
}

/// Result type for core operations
pub type Result<T> = std::result::Result<T, CoreError>;
