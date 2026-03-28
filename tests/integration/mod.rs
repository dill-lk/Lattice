//! Integration tests for Lattice blockchain
//!
//! This module contains comprehensive integration tests that validate
//! the interaction between different components of the Lattice blockchain.

mod block_tests;
mod crypto_tests;
mod consensus_tests;

// Re-export test utilities if needed
pub use lattice_core::{Block, BlockHeader, Transaction, Address, State};
pub use lattice_crypto::{Keypair, sha3_256};
pub use lattice_consensus::{PoWConfig, DifficultyAdjuster};

