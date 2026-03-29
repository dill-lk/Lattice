//! Advanced P2P Sharding for network scalability
//!
//! Implements data sharding to distribute load across network peers

use lattice_core::{Address, BlockHeight, Hash};
use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

/// Shard configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardConfig {
    /// Number of shards
    pub shard_count: u16,
    /// Nodes per shard (minimum)
    pub min_nodes_per_shard: usize,
    /// Replication factor
    pub replication_factor: usize,
    /// Reshard threshold (when to trigger resharding)
    pub reshard_threshold: f64,
}

impl Default for ShardConfig {
    fn default() -> Self {
        Self {
            shard_count: 64,              // 64 shards by default
            min_nodes_per_shard: 4,       // At least 4 nodes per shard
            replication_factor: 3,         // 3x replication
            reshard_threshold: 0.2,        // Reshard if load imbalance > 20%
        }
    }
}

/// Shard identifier
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct ShardId(pub u16);

impl ShardId {
    /// Create shard ID from hash
    pub fn from_hash(hash: &Hash, shard_count: u16) -> Self {
        // Use first 2 bytes of hash
        let value = u16::from_be_bytes([hash[0], hash[1]]);
        Self(value % shard_count)
    }

    /// Create shard ID from address
    pub fn from_address(address: &Address, shard_count: u16) -> Self {
        let bytes = address.as_bytes();
        let value = u16::from_be_bytes([bytes[0], bytes[1]]);
        Self(value % shard_count)
    }

    /// Get shard ID value
    pub fn value(&self) -> u16 {
        self.0
    }
}

/// Peer node information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardPeer {
    /// Peer ID
    pub peer_id: String,
    /// Shards this peer belongs to
    pub shards: HashSet<ShardId>,
    /// Peer capacity (relative score)
    pub capacity: f64,
    /// Number of connections
    pub connections: usize,
    /// Last seen timestamp
    pub last_seen: u64,
}

impl ShardPeer {
    pub fn new(peer_id: String, capacity: f64) -> Self {
        Self {
            peer_id,
            shards: HashSet::new(),
            capacity,
            connections: 0,
            last_seen: 0,
        }
    }

    /// Check if peer is in shard
    pub fn is_in_shard(&self, shard_id: ShardId) -> bool {
        self.shards.contains(&shard_id)
    }

    /// Add peer to shard
    pub fn join_shard(&mut self, shard_id: ShardId) {
        self.shards.insert(shard_id);
    }

    /// Remove peer from shard
    pub fn leave_shard(&mut self, shard_id: ShardId) {
        self.shards.remove(&shard_id);
    }
}

/// Shard information
#[derive(Debug, Clone)]
pub struct Shard {
    /// Shard ID
    pub id: ShardId,
    /// Peers in this shard
    pub peers: HashSet<String>,
    /// Load metric (transactions per second)
    pub load: f64,
    /// Last block height synced
    pub last_synced_height: BlockHeight,
}

impl Shard {
    pub fn new(id: ShardId) -> Self {
        Self {
            id,
            peers: HashSet::new(),
            load: 0.0,
            last_synced_height: 0,
        }
    }

    /// Add peer to shard
    pub fn add_peer(&mut self, peer_id: String) {
        self.peers.insert(peer_id);
    }

    /// Remove peer from shard
    pub fn remove_peer(&mut self, peer_id: &str) {
        self.peers.remove(peer_id);
    }

    /// Get peer count
    pub fn peer_count(&self) -> usize {
        self.peers.len()
    }

    /// Update load metric
    pub fn update_load(&mut self, tps: f64) {
        // Exponential moving average
        self.load = self.load * 0.7 + tps * 0.3;
    }
}

/// Shard manager
pub struct ShardManager {
    /// Configuration
    config: ShardConfig,
    /// All shards
    shards: HashMap<ShardId, Shard>,
    /// All peers
    peers: HashMap<String, ShardPeer>,
    /// Shard assignments (address -> shard)
    address_assignments: HashMap<Address, ShardId>,
}

impl ShardManager {
    /// Create new shard manager
    pub fn new(config: ShardConfig) -> Self {
        let mut shards = HashMap::new();
        
        // Initialize shards
        for i in 0..config.shard_count {
            shards.insert(ShardId(i), Shard::new(ShardId(i)));
        }
        
        Self {
            config,
            shards,
            peers: HashMap::new(),
            address_assignments: HashMap::new(),
        }
    }

    /// Register a new peer
    pub fn register_peer(&mut self, peer_id: String, capacity: f64) {
        let peer = ShardPeer::new(peer_id.clone(), capacity);
        self.peers.insert(peer_id, peer);
    }

    /// Assign peer to optimal shards
    pub fn assign_peer_to_shards(&mut self, peer_id: &str) -> Vec<ShardId> {
        let mut assigned = Vec::new();
        
        // Find shards that need more peers
        let mut shard_needs: Vec<_> = self.shards
            .values()
            .filter(|s| s.peer_count() < self.config.min_nodes_per_shard)
            .map(|s| s.id)
            .collect();
        
        // Sort by load (assign to least loaded first)
        shard_needs.sort_by(|a, b| {
            let load_a = self.shards.get(a).map(|s| s.load).unwrap_or(0.0);
            let load_b = self.shards.get(b).map(|s| s.load).unwrap_or(0.0);
            load_a.partial_cmp(&load_b).unwrap()
        });
        
        // Assign to shards
        for shard_id in shard_needs.iter().take(self.config.replication_factor) {
            if let Some(peer) = self.peers.get_mut(peer_id) {
                peer.join_shard(*shard_id);
                assigned.push(*shard_id);
            }
            
            if let Some(shard) = self.shards.get_mut(shard_id) {
                shard.add_peer(peer_id.to_string());
            }
        }
        
        assigned
    }

    /// Get shard for address
    pub fn get_shard_for_address(&mut self, address: &Address) -> ShardId {
        if let Some(&shard_id) = self.address_assignments.get(address) {
            return shard_id;
        }
        
        let shard_id = ShardId::from_address(address, self.config.shard_count);
        self.address_assignments.insert(address.clone(), shard_id);
        shard_id
    }

    /// Get shard for transaction hash
    pub fn get_shard_for_transaction(&self, tx_hash: &Hash) -> ShardId {
        ShardId::from_hash(tx_hash, self.config.shard_count)
    }

    /// Get peers in shard
    pub fn get_shard_peers(&self, shard_id: ShardId) -> Vec<String> {
        self.shards
            .get(&shard_id)
            .map(|s| s.peers.iter().cloned().collect())
            .unwrap_or_default()
    }

    /// Update shard load
    pub fn update_shard_load(&mut self, shard_id: ShardId, tps: f64) {
        if let Some(shard) = self.shards.get_mut(&shard_id) {
            shard.update_load(tps);
        }
    }

    /// Check if resharding is needed
    pub fn needs_resharding(&self) -> bool {
        let loads: Vec<f64> = self.shards.values().map(|s| s.load).collect();
        
        if loads.is_empty() {
            return false;
        }
        
        let avg_load: f64 = loads.iter().sum::<f64>() / loads.len() as f64;
        let max_load = loads.iter().fold(0.0_f64, |a, &b| a.max(b));
        let min_load = loads.iter().fold(f64::MAX, |a, &b| a.min(b));
        
        // Check if imbalance exceeds threshold
        if avg_load > 0.0 {
            let imbalance = (max_load - min_load) / avg_load;
            imbalance > self.config.reshard_threshold
        } else {
            false
        }
    }

    /// Perform resharding (rebalance load)
    pub fn reshard(&mut self) -> ReshardReport {
        let mut moved_addresses = 0;
        let moved_peers = 0;
        
        // Calculate target load per shard
        let total_load: f64 = self.shards.values().map(|s| s.load).sum();
        let target_load = total_load / self.shards.len() as f64;
        
        // Find overloaded and underloaded shards
        let mut overloaded: Vec<_> = self.shards
            .values()
            .filter(|s| s.load > target_load * 1.1)
            .map(|s| s.id)
            .collect();
        
        let mut underloaded: Vec<_> = self.shards
            .values()
            .filter(|s| s.load < target_load * 0.9)
            .map(|s| s.id)
            .collect();
        
        // Move addresses from overloaded to underloaded shards
        while !overloaded.is_empty() && !underloaded.is_empty() {
            let from_shard = overloaded[0];
            let to_shard = underloaded[0];
            
            // Find addresses in overloaded shard
            let addresses_to_move: Vec<Address> = self.address_assignments
                .iter()
                .filter(|(_, &shard_id)| shard_id == from_shard)
                .take(10)  // Move in batches
                .map(|(addr, _)| addr.clone())
                .collect();
            
            for addr in addresses_to_move {
                self.address_assignments.insert(addr, to_shard);
                moved_addresses += 1;
            }
            
            // Transfer load directly from overloaded to underloaded shard to converge
            let from_load = self.shards.get(&from_shard).map(|s| s.load).unwrap_or(0.0);
            let to_load = self.shards.get(&to_shard).map(|s| s.load).unwrap_or(0.0);
            let transfer = (from_load - target_load).min(target_load - to_load).max(0.0);
            
            if let Some(shard) = self.shards.get_mut(&from_shard) {
                shard.load -= transfer;
            }
            if let Some(shard) = self.shards.get_mut(&to_shard) {
                shard.load += transfer;
            }
            
            // Update lists
            overloaded.retain(|&id| {
                self.shards.get(&id).map(|s| s.load > target_load * 1.1).unwrap_or(false)
            });
            underloaded.retain(|&id| {
                self.shards.get(&id).map(|s| s.load < target_load * 0.9).unwrap_or(false)
            });
        }
        
        ReshardReport {
            addresses_moved: moved_addresses,
            peers_reassigned: moved_peers,
            final_imbalance: self.calculate_imbalance(),
        }
    }

    /// Calculate load imbalance
    fn calculate_imbalance(&self) -> f64 {
        let loads: Vec<f64> = self.shards.values().map(|s| s.load).collect();
        
        if loads.is_empty() {
            return 0.0;
        }
        
        let avg_load: f64 = loads.iter().sum::<f64>() / loads.len() as f64;
        let max_load = loads.iter().fold(0.0_f64, |a, &b| a.max(b));
        let min_load = loads.iter().fold(f64::MAX, |a, &b| a.min(b));
        
        if avg_load > 0.0 {
            (max_load - min_load) / avg_load
        } else {
            0.0
        }
    }

    /// Get sharding statistics
    pub fn stats(&self) -> ShardingStats {
        let total_peers: usize = self.peers.len();
        let total_shards = self.shards.len();
        let avg_peers_per_shard = if total_shards > 0 {
            self.shards.values().map(|s| s.peer_count()).sum::<usize>() as f64 / total_shards as f64
        } else {
            0.0
        };
        
        ShardingStats {
            total_shards,
            total_peers,
            avg_peers_per_shard,
            addresses_assigned: self.address_assignments.len(),
            load_imbalance: self.calculate_imbalance(),
        }
    }
}

/// Resharding report
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ReshardReport {
    pub addresses_moved: usize,
    pub peers_reassigned: usize,
    pub final_imbalance: f64,
}

/// Sharding statistics
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ShardingStats {
    pub total_shards: usize,
    pub total_peers: usize,
    pub avg_peers_per_shard: f64,
    pub addresses_assigned: usize,
    pub load_imbalance: f64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_shard_id_from_hash() {
        let hash = [1u8; 32];
        let shard_id = ShardId::from_hash(&hash, 64);
        assert!(shard_id.value() < 64);
    }

    #[test]
    fn test_shard_manager() {
        let config = ShardConfig::default();
        let mut manager = ShardManager::new(config);
        
        // Register peers
        manager.register_peer("peer1".to_string(), 1.0);
        manager.register_peer("peer2".to_string(), 1.0);
        
        // Assign to shards
        let shards1 = manager.assign_peer_to_shards("peer1");
        assert!(!shards1.is_empty());
        
        let stats = manager.stats();
        assert_eq!(stats.total_peers, 2);
    }

    #[test]
    fn test_resharding() {
        let config = ShardConfig {
            shard_count: 4,
            reshard_threshold: 0.2,
            ..Default::default()
        };
        let mut manager = ShardManager::new(config);
        
        // Simulate load imbalance
        manager.update_shard_load(ShardId(0), 100.0);
        manager.update_shard_load(ShardId(1), 10.0);
        
        assert!(manager.needs_resharding());
        
        let report = manager.reshard();
        assert!(report.final_imbalance < 0.5);
    }
}
