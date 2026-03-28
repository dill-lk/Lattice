//! Peer management for Lattice network
//!
//! Handles peer discovery, connection management, and peer scoring.

use dashmap::DashMap;
use libp2p::{
    Multiaddr, PeerId,
};
use parking_lot::RwLock;
use rand::seq::SliceRandom;
use std::collections::HashSet;
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Peer connection state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PeerState {
    /// Peer is unknown/disconnected
    Disconnected,
    /// Connection is being established
    Connecting,
    /// Peer is connected
    Connected,
    /// Peer is banned
    Banned,
}

/// Peer reputation score
#[derive(Debug, Clone, Copy)]
pub struct PeerScore {
    /// Current score (higher is better)
    pub score: i32,
    /// Number of successful interactions
    pub successes: u32,
    /// Number of failed interactions
    pub failures: u32,
    /// Last update time
    pub last_updated: Instant,
}

impl Default for PeerScore {
    fn default() -> Self {
        Self {
            score: 100,
            successes: 0,
            failures: 0,
            last_updated: Instant::now(),
        }
    }
}

impl PeerScore {
    /// Increase score for good behavior
    pub fn reward(&mut self, points: i32) {
        self.score = self.score.saturating_add(points).min(1000);
        self.successes += 1;
        self.last_updated = Instant::now();
    }

    /// Decrease score for bad behavior
    pub fn penalize(&mut self, points: i32) {
        self.score = self.score.saturating_sub(points).max(-1000);
        self.failures += 1;
        self.last_updated = Instant::now();
    }

    /// Check if peer should be banned
    pub fn should_ban(&self) -> bool {
        self.score < -500
    }
}

/// Information about a peer
#[derive(Debug, Clone)]
pub struct PeerInfo {
    /// Peer ID
    pub peer_id: PeerId,
    /// Known addresses for this peer
    pub addresses: Vec<Multiaddr>,
    /// Current connection state
    pub state: PeerState,
    /// Peer score
    pub score: PeerScore,
    /// When we first saw this peer
    pub first_seen: Instant,
    /// Last successful interaction
    pub last_seen: Option<Instant>,
    /// Protocol version
    pub protocol_version: Option<String>,
    /// Best known block height
    pub best_height: Option<u64>,
}

impl PeerInfo {
    /// Create new peer info
    pub fn new(peer_id: PeerId) -> Self {
        Self {
            peer_id,
            addresses: Vec::new(),
            state: PeerState::Disconnected,
            score: PeerScore::default(),
            first_seen: Instant::now(),
            last_seen: None,
            protocol_version: None,
            best_height: None,
        }
    }

    /// Add an address for this peer
    pub fn add_address(&mut self, addr: Multiaddr) {
        if !self.addresses.contains(&addr) {
            self.addresses.push(addr);
        }
    }
}

/// Configuration for peer management
#[derive(Debug, Clone)]
pub struct PeerConfig {
    /// Maximum number of connected peers
    pub max_peers: usize,
    /// Maximum number of inbound connections
    pub max_inbound: usize,
    /// Maximum number of outbound connections
    pub max_outbound: usize,
    /// Ban duration for misbehaving peers
    pub ban_duration: Duration,
    /// Minimum score to maintain connection
    pub min_score: i32,
    /// Enable mDNS for local peer discovery
    pub enable_mdns: bool,
    /// Bootstrap peers
    pub bootstrap_peers: Vec<(PeerId, Multiaddr)>,
}

impl Default for PeerConfig {
    fn default() -> Self {
        Self {
            max_peers: 50,
            max_inbound: 25,
            max_outbound: 25,
            ban_duration: Duration::from_secs(3600), // 1 hour
            min_score: -100,
            enable_mdns: true,
            bootstrap_peers: Vec::new(),
        }
    }
}

/// Manages peer connections and discovery
pub struct PeerManager {
    /// Configuration
    config: PeerConfig,
    /// All known peers
    peers: DashMap<PeerId, PeerInfo>,
    /// Currently connected peers
    connected: Arc<RwLock<HashSet<PeerId>>>,
    /// Banned peers with expiry time
    banned: DashMap<PeerId, Instant>,
    /// Inbound connection count
    inbound_count: Arc<RwLock<usize>>,
    /// Outbound connection count
    outbound_count: Arc<RwLock<usize>>,
}

impl PeerManager {
    /// Create a new peer manager
    pub fn new(config: PeerConfig) -> Self {
        Self {
            config,
            peers: DashMap::new(),
            connected: Arc::new(RwLock::new(HashSet::new())),
            banned: DashMap::new(),
            inbound_count: Arc::new(RwLock::new(0)),
            outbound_count: Arc::new(RwLock::new(0)),
        }
    }

    /// Check if we can accept a new connection
    pub fn can_accept_connection(&self, inbound: bool) -> bool {
        let connected = self.connected.read().len();
        if connected >= self.config.max_peers {
            return false;
        }

        if inbound {
            *self.inbound_count.read() < self.config.max_inbound
        } else {
            *self.outbound_count.read() < self.config.max_outbound
        }
    }

    /// Register a new peer
    pub fn add_peer(&self, peer_id: PeerId, addr: Option<Multiaddr>) {
        if self.is_banned(&peer_id) {
            return;
        }

        self.peers
            .entry(peer_id)
            .or_insert_with(|| PeerInfo::new(peer_id))
            .value_mut()
            .addresses
            .extend(addr);
    }

    /// Handle peer connected event
    pub fn on_peer_connected(&self, peer_id: PeerId, inbound: bool) {
        if let Some(mut peer) = self.peers.get_mut(&peer_id) {
            peer.state = PeerState::Connected;
            peer.last_seen = Some(Instant::now());
        } else {
            let mut info = PeerInfo::new(peer_id);
            info.state = PeerState::Connected;
            info.last_seen = Some(Instant::now());
            self.peers.insert(peer_id, info);
        }

        self.connected.write().insert(peer_id);

        if inbound {
            *self.inbound_count.write() += 1;
        } else {
            *self.outbound_count.write() += 1;
        }

        tracing::info!(?peer_id, inbound, "Peer connected");
    }

    /// Handle peer disconnected event
    pub fn on_peer_disconnected(&self, peer_id: &PeerId, inbound: bool) {
        if let Some(mut peer) = self.peers.get_mut(peer_id) {
            peer.state = PeerState::Disconnected;
        }

        self.connected.write().remove(peer_id);

        if inbound {
            let mut count = self.inbound_count.write();
            *count = count.saturating_sub(1);
        } else {
            let mut count = self.outbound_count.write();
            *count = count.saturating_sub(1);
        }

        tracing::info!(?peer_id, "Peer disconnected");
    }

    /// Get peer info
    pub fn get_peer(&self, peer_id: &PeerId) -> Option<PeerInfo> {
        self.peers.get(peer_id).map(|p| p.clone())
    }

    /// Update peer's best height
    pub fn update_peer_height(&self, peer_id: &PeerId, height: u64) {
        if let Some(mut peer) = self.peers.get_mut(peer_id) {
            peer.best_height = Some(height);
            peer.last_seen = Some(Instant::now());
        }
    }

    /// Reward a peer for good behavior
    pub fn reward_peer(&self, peer_id: &PeerId, points: i32) {
        if let Some(mut peer) = self.peers.get_mut(peer_id) {
            peer.score.reward(points);
        }
    }

    /// Penalize a peer for bad behavior
    pub fn penalize_peer(&self, peer_id: &PeerId, points: i32) {
        if let Some(mut peer) = self.peers.get_mut(peer_id) {
            peer.score.penalize(points);
            if peer.score.should_ban() {
                drop(peer);
                self.ban_peer(peer_id);
            }
        }
    }

    /// Ban a peer
    pub fn ban_peer(&self, peer_id: &PeerId) {
        let expiry = Instant::now() + self.config.ban_duration;
        self.banned.insert(*peer_id, expiry);

        if let Some(mut peer) = self.peers.get_mut(peer_id) {
            peer.state = PeerState::Banned;
        }

        tracing::warn!(?peer_id, "Peer banned");
    }

    /// Check if a peer is banned
    pub fn is_banned(&self, peer_id: &PeerId) -> bool {
        if let Some(expiry) = self.banned.get(peer_id) {
            if Instant::now() < *expiry {
                return true;
            }
            // Ban expired, remove it
            drop(expiry);
            self.banned.remove(peer_id);
        }
        false
    }

    /// Get list of connected peers
    pub fn connected_peers(&self) -> Vec<PeerId> {
        self.connected.read().iter().copied().collect()
    }

    /// Get number of connected peers
    pub fn connected_count(&self) -> usize {
        self.connected.read().len()
    }

    /// Get peers with best known height above threshold
    pub fn peers_above_height(&self, height: u64) -> Vec<PeerId> {
        self.connected
            .read()
            .iter()
            .filter(|peer_id| {
                self.peers
                    .get(peer_id)
                    .and_then(|p| p.best_height)
                    .map(|h| h > height)
                    .unwrap_or(false)
            })
            .copied()
            .collect()
    }

    /// Get peers sorted by score (best first)
    pub fn best_peers(&self, count: usize) -> Vec<PeerId> {
        let mut peers: Vec<_> = self
            .connected
            .read()
            .iter()
            .filter_map(|peer_id| {
                self.peers
                    .get(peer_id)
                    .map(|p| (*peer_id, p.score.score))
            })
            .collect();

        peers.sort_by(|a, b| b.1.cmp(&a.1));
        peers.into_iter().take(count).map(|(id, _)| id).collect()
    }

    /// Get random connected peers
    pub fn random_peers(&self, count: usize) -> Vec<PeerId> {
        let connected: Vec<_> = self.connected.read().iter().copied().collect();
        let mut rng = rand::thread_rng();
        connected
            .choose_multiple(&mut rng, count.min(connected.len()))
            .copied()
            .collect()
    }

    /// Handle mDNS discovered peer
    pub fn on_mdns_discovered(&self, peer_id: PeerId, addr: Multiaddr) {
        tracing::debug!(?peer_id, %addr, "mDNS discovered peer");
        self.add_peer(peer_id, Some(addr));
    }

    /// Handle mDNS expired peer
    pub fn on_mdns_expired(&self, peer_id: &PeerId) {
        tracing::debug!(?peer_id, "mDNS peer expired");
        // Don't remove, just log - peer might still be connected
    }

    /// Prune low-score peers to make room for better ones
    pub fn prune_low_score_peers(&self) -> Vec<PeerId> {
        let connected = self.connected.read();
        if connected.len() < self.config.max_peers {
            return Vec::new();
        }

        let mut peers_with_scores: Vec<_> = connected
            .iter()
            .filter_map(|peer_id| {
                self.peers
                    .get(peer_id)
                    .map(|p| (*peer_id, p.score.score))
            })
            .filter(|(_, score)| *score < self.config.min_score)
            .collect();

        peers_with_scores.sort_by(|a, b| a.1.cmp(&b.1));
        peers_with_scores
            .into_iter()
            .take(5) // Prune at most 5 at a time
            .map(|(id, _)| id)
            .collect()
    }

    /// Get bootstrap peers
    pub fn bootstrap_peers(&self) -> &[(PeerId, Multiaddr)] {
        &self.config.bootstrap_peers
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn random_peer_id() -> PeerId {
        PeerId::random()
    }

    #[test]
    fn test_peer_score() {
        let mut score = PeerScore::default();
        assert_eq!(score.score, 100);

        score.reward(50);
        assert_eq!(score.score, 150);
        assert_eq!(score.successes, 1);

        score.penalize(200);
        assert_eq!(score.score, -50);
        assert_eq!(score.failures, 1);
    }

    #[test]
    fn test_peer_manager() {
        let config = PeerConfig::default();
        let manager = PeerManager::new(config);

        let peer_id = random_peer_id();
        manager.add_peer(peer_id, None);
        assert!(manager.get_peer(&peer_id).is_some());

        manager.on_peer_connected(peer_id, true);
        assert_eq!(manager.connected_count(), 1);

        manager.on_peer_disconnected(&peer_id, true);
        assert_eq!(manager.connected_count(), 0);
    }

    #[test]
    fn test_peer_banning() {
        let config = PeerConfig::default();
        let manager = PeerManager::new(config);

        let peer_id = random_peer_id();
        assert!(!manager.is_banned(&peer_id));

        manager.ban_peer(&peer_id);
        assert!(manager.is_banned(&peer_id));
    }

    #[test]
    fn test_connection_limits() {
        let mut config = PeerConfig::default();
        config.max_peers = 2;
        config.max_inbound = 1;
        config.max_outbound = 1;

        let manager = PeerManager::new(config);

        assert!(manager.can_accept_connection(true));
        assert!(manager.can_accept_connection(false));

        let peer1 = random_peer_id();
        manager.on_peer_connected(peer1, true);
        assert!(!manager.can_accept_connection(true));
        assert!(manager.can_accept_connection(false));

        let peer2 = random_peer_id();
        manager.on_peer_connected(peer2, false);
        assert!(!manager.can_accept_connection(true));
        assert!(!manager.can_accept_connection(false));
    }
}
