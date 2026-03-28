//! RPC error types

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ErrorCode {
    /// Parse error - Invalid JSON
    ParseError = -32700,
    /// Invalid Request - JSON is not a valid Request object
    InvalidRequest = -32600,
    /// Method not found
    MethodNotFound = -32601,
    /// Invalid params
    InvalidParams = -32602,
    /// Internal error
    InternalError = -32603,
    /// Block not found
    BlockNotFound = -32001,
    /// Transaction not found
    TransactionNotFound = -32002,
    /// Invalid transaction
    InvalidTransaction = -32003,
    /// Execution error
    ExecutionError = -32004,
}

impl ErrorCode {
    pub fn code(&self) -> i32 {
        *self as i32
    }

    pub fn message(&self) -> &'static str {
        match self {
            ErrorCode::ParseError => "Parse error",
            ErrorCode::InvalidRequest => "Invalid Request",
            ErrorCode::MethodNotFound => "Method not found",
            ErrorCode::InvalidParams => "Invalid params",
            ErrorCode::InternalError => "Internal error",
            ErrorCode::BlockNotFound => "Block not found",
            ErrorCode::TransactionNotFound => "Transaction not found",
            ErrorCode::InvalidTransaction => "Invalid transaction",
            ErrorCode::ExecutionError => "Execution error",
        }
    }
}

/// JSON-RPC 2.0 error object
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcError {
    pub code: i32,
    pub message: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<serde_json::Value>,
}

impl RpcError {
    pub fn new(code: ErrorCode, message: impl Into<String>) -> Self {
        Self {
            code: code.code(),
            message: message.into(),
            data: None,
        }
    }

    pub fn with_data(mut self, data: serde_json::Value) -> Self {
        self.data = Some(data);
        self
    }

    pub fn parse_error() -> Self {
        Self::new(ErrorCode::ParseError, ErrorCode::ParseError.message())
    }

    pub fn invalid_request() -> Self {
        Self::new(ErrorCode::InvalidRequest, ErrorCode::InvalidRequest.message())
    }

    pub fn method_not_found() -> Self {
        Self::new(ErrorCode::MethodNotFound, ErrorCode::MethodNotFound.message())
    }

    pub fn invalid_params(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidParams, msg)
    }

    pub fn internal_error(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::InternalError, msg)
    }

    pub fn block_not_found() -> Self {
        Self::new(ErrorCode::BlockNotFound, ErrorCode::BlockNotFound.message())
    }

    pub fn transaction_not_found() -> Self {
        Self::new(
            ErrorCode::TransactionNotFound,
            ErrorCode::TransactionNotFound.message(),
        )
    }

    pub fn invalid_transaction(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::InvalidTransaction, msg)
    }

    pub fn execution_error(msg: impl Into<String>) -> Self {
        Self::new(ErrorCode::ExecutionError, msg)
    }
}

/// Result type for RPC operations
pub type Result<T> = std::result::Result<T, RpcError>;

/// Crate-level error type for library errors
#[derive(Debug, thiserror::Error)]
pub enum Error {
    #[error("server error: {0}")]
    Server(String),

    #[error("bind error: {0}")]
    Bind(#[from] std::io::Error),

    #[error("json error: {0}")]
    Json(#[from] serde_json::Error),
}
