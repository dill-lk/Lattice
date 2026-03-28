//! Chain synchronization for Lattice
//!
//! Implements header-first sync with parallel block downloading.

use crate::error::{NetworkError, Result};
use crate::peer::PeerManager;
use crate::protocol::{SyncRequest, SyncResponse};
use dashmap::DashMap;
use lattice_core::{Block, BlockHeader, BlockHeight, Hash};
use libp2p::{request_response::OutboundRequestId, PeerId};
use parking_lot::{Mutex, RwLock};
use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Sync state
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncState {
    /// Not syncing, at chain tip
    Idle,
    /// Finding common ancestor with network
    FindingAncestor,
    /// Downloading headers
    DownloadingHeaders,
    /// Downloading block bodies
    DownloadingBlocks,
    /// Importing blocks
    Importing,
}

/// Configuration for chain sync
#[derive(Debug, Clone)]
pub struct SyncConfig {
    /// Maximum headers to request at once
    pub max_headers_fetch: u32,
    /// Maximum blocks to request at once
    pub max_blocks_fetch: usize,
    /// Number of parallel block download requests
    pub parallel_downloads: usize,
    /// Request timeout
    pub request_timeout: Duration,
    /// How often to check sync progress
    pub progress_check_interval: Duration,
    /// Minimum peers before syncing
    pub min_peers: usize,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            max_headers_fetch: 2000,
            max_blocks_fetch: 128,
            parallel_downloads: 4,
            request_timeout: Duration::from_secs(30),
            progress_check_interval: Duration::from_secs(5),
            min_peers: 1,
        }
    }
}

/// A pending request
#[derive(Debug)]
struct PendingRequest {
    /// When the request was sent
    sent_at: Instant,
    /// The peer we sent the request to
    peer: PeerId,
    /// What kind of request
    kind: RequestKind,
}

#[derive(Debug, Clone)]
enum RequestKind {
    Status,
    Headers { start_hash: Hash },
    Blocks { hashes: Vec<Hash> },
}

/// Block download task
#[derive(Debug, Clone)]
struct BlockTask {
    /// Block hash to download
    hash: Hash,
    /// Expected height (for ordering)
    height: BlockHeight,
    /// Parent hash (for validation)
    parent_hash: Hash,
}

/// Chain synchronization manager
pub struct ChainSync {
    /// Configuration
    config: SyncConfig,
    /// Current sync state
    state: Arc<RwLock<SyncState>>,
    /// Our current best height
    local_height: Arc<RwLock<BlockHeight>>,
    /// Our current best hash
    local_hash: Arc<RwLock<Hash>>,
    /// Genesis hash (for validation)
    genesis_hash: Hash,
    /// Headers we've downloaded but not yet processed
    header_queue: Arc<Mutex<VecDeque<BlockHeader>>>,
    /// Blocks waiting to be imported
    block_queue: Arc<Mutex<VecDeque<Block>>>,
    /// Block download tasks
    download_tasks: Arc<Mutex<VecDeque<BlockTask>>>,
    /// Blocks currently being downloaded (hash -> peer)
    downloading: DashMap<Hash, PeerId>,
    /// Pending requests (request_id -> info)
    pending_requests: DashMap<OutboundRequestId, PendingRequest>,
    /// Received blocks waiting to be ordered
    received_blocks: DashMap<Hash, Block>,
    /// Known block heights from headers
    known_heights: DashMap<Hash, BlockHeight>,
    /// Headers we've seen (for dedup)
    seen_headers: Arc<RwLock<HashSet<Hash>>>,
    /// Sync target height (from peers)
    target_height: Arc<RwLock<BlockHeight>>,
    /// Last progress time
    last_progress: Arc<RwLock<Instant>>,
}

impl ChainSync {
    /// Create a new chain sync manager
    pub fn new(
        config: SyncConfig,
        genesis_hash: Hash,
        local_height: BlockHeight,
        local_hash: Hash,
    ) -> Self {
        Self {
            config,
            state: Arc::new(RwLock::new(SyncState::Idle)),
            local_height: Arc::new(RwLock::new(local_height)),
            local_hash: Arc::new(RwLock::new(local_hash)),
            genesis_hash,
            header_queue: Arc::new(Mutex::new(VecDeque::new())),
            block_queue: Arc::new(Mutex::new(VecDeque::new())),
            download_tasks: Arc::new(Mutex::new(VecDeque::new())),
            downloading: DashMap::new(),
            pending_requests: DashMap::new(),
            received_blocks: DashMap::new(),
            known_heights: DashMap::new(),
            seen_headers: Arc::new(RwLock::new(HashSet::new())),
            target_height: Arc::new(RwLock::new(0)),
            last_progress: Arc::new(RwLock::new(Instant::now())),
        }
    }

    /// Get current sync state
    pub fn state(&self) -> SyncState {
        *self.state.read()
    }

    /// Check if we're currently syncing
    pub fn is_syncing(&self) -> bool {
        !matches!(self.state(), SyncState::Idle)
    }

    /// Get sync progress (0.0 - 1.0)
    pub fn progress(&self) -> f64 {
        let local = *self.local_height.read();
        let target = *self.target_height.read();

        if target == 0 || target <= local {
            return 1.0;
        }

        local as f64 / target as f64
    }

    /// Get current local height
    pub fn local_height(&self) -> BlockHeight {
        *self.local_height.read()
    }

    /// Get target height
    pub fn target_height(&self) -> BlockHeight {
        *self.target_height.read()
    }

    /// Update local chain state
    pub fn update_local_state(&self, height: BlockHeight, hash: Hash) {
        *self.local_height.write() = height;
        *self.local_hash.write() = hash;
        *self.last_progress.write() = Instant::now();
    }

    /// Start sync with the network
    pub fn start_sync(&self, peer_manager: &PeerManager) -> Vec<(PeerId, SyncRequest)> {
        let connected = peer_manager.connected_count();
        if connected < self.config.min_peers {
            tracing::debug!(
                "Not enough peers to sync ({}/{})",
                connected,
                self.config.min_peers
            );
            return Vec::new();
        }

        *self.state.write() = SyncState::FindingAncestor;

        // Request status from all connected peers
        peer_manager
            .connected_peers()
            .into_iter()
            .map(|peer| (peer, SyncRequest::GetStatus))
            .collect()
    }

    /// Handle a status response
    pub fn on_status_response(
        &self,
        peer: PeerId,
        best_height: BlockHeight,
        best_hash: Hash,
        genesis_hash: Hash,
        peer_manager: &PeerManager,
    ) -> Option<(PeerId, SyncRequest)> {
        // Verify genesis matches
        if genesis_hash != self.genesis_hash {
            tracing::warn!(
                ?peer,
                "Peer has different genesis hash, disconnecting"
            );
            peer_manager.penalize_peer(&peer, 100);
            return None;
        }

        // Update peer's best height
        peer_manager.update_peer_height(&peer, best_height);

        let local_height = *self.local_height.read();

        // Update target if peer is ahead
        if best_height > *self.target_height.read() {
            *self.target_height.write() = best_height;
        }

        // If peer is behind or same, nothing to sync from them
        if best_height <= local_height {
            return None;
        }

        // Start header download
        *self.state.write() = SyncState::DownloadingHeaders;

        let start_hash = *self.local_hash.read();
        Some((
            peer,
            SyncRequest::GetHeaders {
                start_hash,
                max_headers: self.config.max_headers_fetch,
            },
        ))
    }

    /// Handle headers response
    pub fn on_headers_response(
        &self,
        peer: PeerId,
        headers: Vec<BlockHeader>,
        peer_manager: &PeerManager,
    ) -> Vec<(PeerId, SyncRequest)> {
        if headers.is_empty() {
            // Finished downloading headers
            if self.download_tasks.lock().is_empty() && self.header_queue.lock().is_empty() {
                *self.state.write() = SyncState::Idle;
            }
            return Vec::new();
        }

        tracing::debug!(
            count = headers.len(),
            "Received {} headers from {:?}",
            headers.len(),
            peer
        );

        peer_manager.reward_peer(&peer, 1);

        // Validate and queue headers
        let mut queue = self.header_queue.lock();
        let mut seen = self.seen_headers.write();
        let mut tasks = self.download_tasks.lock();

        let mut last_hash = *self.local_hash.read();
        let mut last_height = *self.local_height.read();

        for header in headers.iter() {
            let hash = header.hash();

            // Skip if already seen
            if seen.contains(&hash) {
                continue;
            }

            // Validate parent link
            if header.prev_hash != last_hash && !queue.is_empty() {
                tracing::warn!(?peer, "Invalid header chain from peer");
                peer_manager.penalize_peer(&peer, 50);
                return Vec::new();
            }

            // Queue header
            seen.insert(hash);
            queue.push_back(header.clone());

            // Create download task
            tasks.push_back(BlockTask {
                hash,
                height: header.height,
                parent_hash: header.prev_hash,
            });

            self.known_heights.insert(hash, header.height);

            last_hash = hash;
            last_height = header.height;
        }

        drop(queue);
        drop(seen);
        drop(tasks);

        *self.last_progress.write() = Instant::now();

        // Request more headers if we got a full batch
        let mut requests = Vec::new();

        if headers.len() as u32 >= self.config.max_headers_fetch {
            let last_header = headers.last().unwrap();
            requests.push((
                peer,
                SyncRequest::GetHeaders {
                    start_hash: last_header.hash(),
                    max_headers: self.config.max_headers_fetch,
                },
            ));
        }

        // Start block downloads
        requests.extend(self.dispatch_block_downloads(peer_manager));

        if !requests.is_empty() {
            *self.state.write() = SyncState::DownloadingBlocks;
        }

        requests
    }

    /// Dispatch block download requests
    fn dispatch_block_downloads(&self, peer_manager: &PeerManager) -> Vec<(PeerId, SyncRequest)> {
        let mut requests = Vec::new();
        let peers = peer_manager.best_peers(self.config.parallel_downloads);

        if peers.is_empty() {
            return requests;
        }

        let mut tasks = self.download_tasks.lock();
        let mut peer_idx = 0;

        while self.downloading.len() < self.config.parallel_downloads && !tasks.is_empty() {
            // Gather hashes for this batch
            let mut batch_hashes = Vec::with_capacity(self.config.max_blocks_fetch);
            while batch_hashes.len() < self.config.max_blocks_fetch {
                if let Some(task) = tasks.pop_front() {
                    if !self.downloading.contains_key(&task.hash) {
                        self.downloading.insert(task.hash, peers[peer_idx]);
                        batch_hashes.push(task.hash);
                    }
                } else {
                    break;
                }
            }

            if !batch_hashes.is_empty() {
                requests.push((
                    peers[peer_idx],
                    SyncRequest::GetBlocks {
                        hashes: batch_hashes,
                    },
                ));
                peer_idx = (peer_idx + 1) % peers.len();
            }
        }

        requests
    }

    /// Handle blocks response
    pub fn on_blocks_response(
        &self,
        peer: PeerId,
        blocks: Vec<Block>,
        peer_manager: &PeerManager,
    ) -> Vec<(PeerId, SyncRequest)> {
        if blocks.is_empty() {
            tracing::warn!(?peer, "Received empty blocks response");
            peer_manager.penalize_peer(&peer, 10);
            return self.dispatch_block_downloads(peer_manager);
        }

        tracing::debug!(
            count = blocks.len(),
            "Received {} blocks from {:?}",
            blocks.len(),
            peer
        );

        peer_manager.reward_peer(&peer, 5);

        // Store received blocks
        for block in blocks {
            let hash = block.hash();
            self.downloading.remove(&hash);
            self.received_blocks.insert(hash, block);
        }

        // Try to queue blocks in order
        self.try_queue_ordered_blocks();

        *self.last_progress.write() = Instant::now();

        // Continue downloading
        self.dispatch_block_downloads(peer_manager)
    }

    /// Try to move blocks from received_blocks to block_queue in correct order
    fn try_queue_ordered_blocks(&self) {
        let mut queue = self.block_queue.lock();
        let mut expected_hash = *self.local_hash.read();

        // Look for blocks that chain from our current position
        loop {
            let mut found = None;

            // Find a block whose parent is expected_hash
            for entry in self.received_blocks.iter() {
                if entry.value().header.prev_hash == expected_hash {
                    found = Some(entry.key().clone());
                    break;
                }
            }

            if let Some(hash) = found {
                if let Some((_, block)) = self.received_blocks.remove(&hash) {
                    expected_hash = block.hash();
                    queue.push_back(block);
                }
            } else {
                break;
            }
        }
    }

    /// Get next block to import
    pub fn next_block_to_import(&self) -> Option<Block> {
        self.block_queue.lock().pop_front()
    }

    /// Check if there are blocks ready to import
    pub fn has_blocks_to_import(&self) -> bool {
        !self.block_queue.lock().is_empty()
    }

    /// Register a pending request
    pub fn register_request(
        &self,
        request_id: OutboundRequestId,
        peer: PeerId,
        request: &SyncRequest,
    ) {
        let kind = match request {
            SyncRequest::GetStatus => RequestKind::Status,
            SyncRequest::GetHeaders { start_hash, .. } => RequestKind::Headers {
                start_hash: *start_hash,
            },
            SyncRequest::GetBlocks { hashes } => RequestKind::Blocks {
                hashes: hashes.clone(),
            },
            SyncRequest::GetPooledTransactions { .. } => return,
        };

        self.pending_requests.insert(
            request_id,
            PendingRequest {
                sent_at: Instant::now(),
                peer,
                kind,
            },
        );
    }

    /// Handle request failure
    pub fn on_request_failed(
        &self,
        request_id: OutboundRequestId,
        peer_manager: &PeerManager,
    ) -> Vec<(PeerId, SyncRequest)> {
        if let Some((_, req)) = self.pending_requests.remove(&request_id) {
            tracing::warn!(
                ?req.peer,
                "Sync request failed"
            );

            peer_manager.penalize_peer(&req.peer, 10);

            // Re-queue failed block downloads
            if let RequestKind::Blocks { hashes } = req.kind {
                for hash in hashes {
                    self.downloading.remove(&hash);
                    if let Some(height) = self.known_heights.get(&hash) {
                        self.download_tasks.lock().push_front(BlockTask {
                            hash,
                            height: *height,
                            parent_hash: [0u8; 32], // We'll revalidate later
                        });
                    }
                }
            }

            // Try to continue with other peers
            return self.dispatch_block_downloads(peer_manager);
        }

        Vec::new()
    }

    /// Remove completed request
    pub fn complete_request(&self, request_id: OutboundRequestId) {
        self.pending_requests.remove(&request_id);
    }

    /// Check for timed out requests
    pub fn check_timeouts(&self, peer_manager: &PeerManager) -> Vec<(PeerId, SyncRequest)> {
        let timeout = self.config.request_timeout;
        let mut timed_out = Vec::new();

        for entry in self.pending_requests.iter() {
            if entry.value().sent_at.elapsed() > timeout {
                timed_out.push(*entry.key());
            }
        }

        let mut requests = Vec::new();
        for request_id in timed_out {
            requests.extend(self.on_request_failed(request_id, peer_manager));
        }

        requests
    }

    /// Handle a new block announcement
    pub fn on_new_block_announcement(
        &self,
        block: &Block,
        peer: PeerId,
        peer_manager: &PeerManager,
    ) -> Option<(PeerId, SyncRequest)> {
        let local_height = *self.local_height.read();

        // If block is far ahead, we need to sync
        if block.height() > local_height + 1 {
            if !self.is_syncing() {
                peer_manager.update_peer_height(&peer, block.height());
                *self.target_height.write() = block.height();
                *self.state.write() = SyncState::DownloadingHeaders;

                return Some((
                    peer,
                    SyncRequest::GetHeaders {
                        start_hash: *self.local_hash.read(),
                        max_headers: self.config.max_headers_fetch,
                    },
                ));
            }
        }

        None
    }

    /// Check if sync has stalled
    pub fn is_stalled(&self) -> bool {
        if !self.is_syncing() {
            return false;
        }

        self.last_progress.read().elapsed() > Duration::from_secs(60)
    }

    /// Reset sync state after stall
    pub fn reset_after_stall(&self) {
        tracing::warn!("Sync stalled, resetting state");

        *self.state.write() = SyncState::Idle;
        self.pending_requests.clear();
        self.downloading.clear();
        self.received_blocks.clear();
        self.download_tasks.lock().clear();
        *self.last_progress.write() = Instant::now();
    }

    /// Get sync statistics
    pub fn stats(&self) -> SyncStats {
        SyncStats {
            state: self.state(),
            local_height: *self.local_height.read(),
            target_height: *self.target_height.read(),
            headers_queued: self.header_queue.lock().len(),
            blocks_queued: self.block_queue.lock().len(),
            blocks_downloading: self.downloading.len(),
            pending_requests: self.pending_requests.len(),
        }
    }
}

/// Sync statistics
#[derive(Debug, Clone)]
pub struct SyncStats {
    /// Current sync state
    pub state: SyncState,
    /// Local chain height
    pub local_height: BlockHeight,
    /// Target height to sync to
    pub target_height: BlockHeight,
    /// Number of headers queued
    pub headers_queued: usize,
    /// Number of blocks ready to import
    pub blocks_queued: usize,
    /// Number of blocks being downloaded
    pub blocks_downloading: usize,
    /// Number of pending requests
    pub pending_requests: usize,
}

impl std::fmt::Display for SyncStats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            f,
            "Sync[{:?}]: {}/{} (headers: {}, blocks: {}, downloading: {}, pending: {})",
            self.state,
            self.local_height,
            self.target_height,
            self.headers_queued,
            self.blocks_queued,
            self.blocks_downloading,
            self.pending_requests,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::{Address, Block};

    fn genesis_hash() -> Hash {
        Block::genesis().hash()
    }

    #[test]
    fn test_sync_state() {
        let sync = ChainSync::new(SyncConfig::default(), genesis_hash(), 0, genesis_hash());

        assert_eq!(sync.state(), SyncState::Idle);
        assert!(!sync.is_syncing());
        assert_eq!(sync.progress(), 1.0);
    }

    #[test]
    fn test_sync_progress() {
        let sync = ChainSync::new(SyncConfig::default(), genesis_hash(), 50, genesis_hash());

        *sync.target_height.write() = 100;

        assert_eq!(sync.progress(), 0.5);
    }

    #[test]
    fn test_update_local_state() {
        let sync = ChainSync::new(SyncConfig::default(), genesis_hash(), 0, genesis_hash());

        let new_hash = [1u8; 32];
        sync.update_local_state(100, new_hash);

        assert_eq!(sync.local_height(), 100);
    }

    #[test]
    fn test_sync_stats() {
        let sync = ChainSync::new(SyncConfig::default(), genesis_hash(), 0, genesis_hash());

        let stats = sync.stats();
        assert_eq!(stats.state, SyncState::Idle);
        assert_eq!(stats.local_height, 0);
        assert_eq!(stats.headers_queued, 0);
        assert_eq!(stats.blocks_queued, 0);
    }
}
