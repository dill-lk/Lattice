//! Lattice Network - P2P networking layer
//!
//! Built on libp2p with gossipsub for block/transaction propagation.
//!
//! # Overview
//!
//! This crate provides the networking layer for Lattice blockchain:
//!
//! - **Peer Management**: Discovery, connection limits, scoring
//! - **Protocol**: Message types, gossipsub + request-response behaviors
//! - **Chain Sync**: Header-first sync with parallel block downloads
//!
//! # Example
//!
//! ```ignore
//! use lattice_network::{NetworkBehavior, PeerManager, ChainSync};
//!
//! // Create network behavior
//! let behavior = NetworkBehavior::new(&keypair, true)?;
//!
//! // Create peer manager
//! let peer_manager = PeerManager::new(PeerConfig::default());
//!
//! // Create chain sync
//! let sync = ChainSync::new(SyncConfig::default(), genesis_hash, 0, genesis_hash);
//! ```

mod error;
mod peer;
mod protocol;
pub mod sharding;
mod sync;

pub use error::{NetworkError, Result};
pub use peer::{PeerConfig, PeerInfo, PeerManager, PeerScore, PeerState};
pub use protocol::{
    GossipMessage, NetworkBehavior, NetworkEvent, SyncCodec, SyncRequest, SyncResponse,
    PROTOCOL_VERSION, SYNC_PROTOCOL, TOPIC_BLOCKS, TOPIC_TRANSACTIONS,
};
pub use sharding::{ShardConfig, ShardId, ShardManager, ShardingStats};
pub use sync::{ChainSync, SyncConfig, SyncState, SyncStats};
