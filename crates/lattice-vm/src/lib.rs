//! Lattice VM - WebAssembly smart contract runtime
//!
//! This crate provides a sandboxed WASM execution environment for smart contracts
//! with gas metering, host functions for blockchain state access, and secure isolation.
//!
//! # Features
//!
//! - **Gas Metering**: All operations consume gas to prevent infinite loops
//! - **Host Functions**: Storage, crypto, and block info accessible from contracts
//! - **Memory Safety**: WASM sandbox prevents unauthorized memory access
//! - **Deterministic Execution**: Same inputs always produce same outputs
//!
//! # Example
//!
//! ```ignore
//! use lattice_vm::{Runtime, BlockContext, GasMeter};
//! use lattice_core::Address;
//!
//! let mut runtime = Runtime::new();
//!
//! // Deploy a contract
//! let deployer = Address::from_bytes([1u8; 20]);
//! let result = runtime.deploy(
//!     wasm_code,
//!     deployer,
//!     0,  // value
//!     1_000_000,  // gas limit
//!     BlockContext::default(),
//!     vec![],  // init data
//! )?;
//!
//! // Call the contract
//! let exec_result = runtime.call(
//!     result.address,
//!     deployer,
//!     0,
//!     vec![],  // call data
//!     1_000_000,
//!     BlockContext::default(),
//! )?;
//! ```

mod error;
mod gas;
mod host;
mod runtime;

pub use error::{Result, VmError};
pub use gas::{GasCosts, GasMeter};
pub use host::{BlockContext, CallContext, HostFunctions, Log};
pub use runtime::{DeploymentResult, ExecutionResult, Runtime};
