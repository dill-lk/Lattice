//! Lattice RPC - JSON-RPC 2.0 API server
//!
//! This crate provides a JSON-RPC 2.0 compliant API server for the Lattice blockchain.
//!
//! # Example
//!
//! ```no_run
//! use lattice_rpc::{RpcServer, RpcConfig};
//!
//! #[tokio::main]
//! async fn main() {
//!     let config = RpcConfig::new("127.0.0.1", 8545);
//!     let server = RpcServer::new(config);
//!     server.run().await.unwrap();
//! }
//! ```
//!
//! # RPC Methods
//!
//! - `lat_blockNumber` - Get latest block height
//! - `lat_getBlockByNumber` - Get block by height
//! - `lat_getBlockByHash` - Get block by hash
//! - `lat_getTransactionByHash` - Get transaction by hash
//! - `lat_getBalance` - Get account balance
//! - `lat_sendRawTransaction` - Submit signed transaction
//! - `lat_getTransactionReceipt` - Get transaction receipt
//! - `lat_call` - Execute read-only contract call
//! - `lat_estimateGas` - Estimate gas for transaction

mod error;
mod handlers;
mod server;
mod types;

pub use error::{Error, ErrorCode, Result, RpcError};
pub use handlers::{ChainState, RpcHandlers};
pub use server::{RpcConfig, RpcServer};
pub use types::{
    BlockNumber, BlockTag, CallRequest, RpcBlock, RpcRequest, RpcResponse, RpcTransaction,
    TransactionReceipt,
};
