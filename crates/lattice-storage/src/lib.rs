//! Lattice Storage - RocksDB-based persistence layer
//!
//! This crate provides persistent storage for the Lattice blockchain:
//! - [`BlockStore`] - Block storage with hash and height indexes
//! - [`StateStore`] - Account state with snapshot/rollback support
//! - [`MempoolStore`] - Transaction pool with fee-based ordering
//!
//! # Architecture
//!
//! All stores use RocksDB with column families for organization:
//! - Each store manages its own column families
//! - Data is serialized using borsh for efficiency
//! - Thread-safe with internal locking where needed
//!
//! # Example
//!
//! ```ignore
//! use lattice_storage::{BlockStore, StateStore, MempoolStore};
//!
//! let blocks = BlockStore::open("./data/blocks")?;
//! let state = StateStore::open("./data/state")?;
//! let mempool = MempoolStore::open("./data/mempool")?;
//! ```

mod blocks;
mod error;
mod mempool;
mod state;

pub use blocks::BlockStore;
pub use error::{Result, StorageError};
pub use mempool::{MempoolConfig, MempoolStats, MempoolStore};
pub use state::StateStore;
