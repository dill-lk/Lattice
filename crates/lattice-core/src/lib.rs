//! Lattice Core - Fundamental blockchain types and logic
//!
//! This crate provides the core primitives for the Lattice blockchain:
//! - [`Block`] - Block structure with header and transactions
//! - [`Transaction`] - Transaction types and signing
//! - [`Address`] - Quantum-resistant addresses
//! - [`State`] - Account state and transitions

mod address;
mod block;
mod error;
mod state;
mod transaction;

pub use address::Address;
pub use block::{Block, BlockHeader};
pub use error::{CoreError, Result};
pub use state::{Account, State};
pub use transaction::{Transaction, TransactionKind};

/// Block height type
pub type BlockHeight = u64;

/// Amount in smallest unit (1 LAT = 10^18 units)
pub type Amount = u128;

/// Timestamp in milliseconds since Unix epoch
pub type Timestamp = u64;

/// Hash type (SHA3-256)
pub type Hash = [u8; 32];

/// Signature type (Dilithium)
pub type Signature = Vec<u8>;

/// Public key type (Dilithium)
pub type PublicKey = Vec<u8>;

/// Network identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, serde::Serialize, serde::Deserialize)]
pub enum Network {
    Mainnet,
    Testnet,
    Devnet,
}

impl Network {
    pub fn chain_id(&self) -> u32 {
        match self {
            Network::Mainnet => 1,
            Network::Testnet => 2,
            Network::Devnet => 3,
        }
    }
}
