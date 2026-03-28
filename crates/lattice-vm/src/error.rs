//! Error types for the Lattice VM

use thiserror::Error;

/// VM execution errors
#[derive(Debug, Error)]
pub enum VmError {
    #[error("gas exhausted: required {required}, available {available}")]
    OutOfGas { required: u64, available: u64 },

    #[error("gas limit exceeded: limit {limit}, requested {requested}")]
    GasLimitExceeded { limit: u64, requested: u64 },

    #[error("invalid gas limit: {0}")]
    InvalidGasLimit(u64),

    #[error("wasm compilation failed: {0}")]
    CompilationError(String),

    #[error("wasm instantiation failed: {0}")]
    InstantiationError(String),

    #[error("wasm execution failed: {0}")]
    ExecutionError(String),

    #[error("invalid wasm module: {0}")]
    InvalidModule(String),

    #[error("memory access out of bounds: offset {offset}, length {length}")]
    MemoryOutOfBounds { offset: u32, length: u32 },

    #[error("storage key too long: {0} bytes (max 256)")]
    StorageKeyTooLong(usize),

    #[error("storage value too long: {0} bytes (max 65536)")]
    StorageValueTooLong(usize),

    #[error("contract not found: {0}")]
    ContractNotFound(String),

    #[error("entry point not found: {0}")]
    EntryPointNotFound(String),

    #[error("invalid contract code")]
    InvalidContractCode,

    #[error("stack overflow")]
    StackOverflow,

    #[error("call depth exceeded: {0}")]
    CallDepthExceeded(u32),

    #[error("revert: {0}")]
    Revert(String),

    #[error("trap: {0}")]
    Trap(String),

    #[error("host function error: {0}")]
    HostError(String),

    #[error("serialization error: {0}")]
    SerializationError(String),

    #[error("invalid address")]
    InvalidAddress,

    #[error("insufficient balance")]
    InsufficientBalance,
}

/// Result type for VM operations
pub type Result<T> = std::result::Result<T, VmError>;
