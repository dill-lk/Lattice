//! Networking smoke tests for sync-manager flow.
//!
//! These tests do not spin up full libp2p transports, but they exercise the
//! request / response synchronization path that the node networking layer uses.

use lattice_core::{Address, Block, BlockHeader};
use lattice_network::{ChainSync, PeerConfig, PeerManager, SyncConfig, SyncRequest};
use libp2p::PeerId;

fn make_block(height: u64, prev_hash: [u8; 32], timestamp: u64) -> Block {
    Block {
        header: BlockHeader {
            version: 1,
            height,
            prev_hash,
            tx_root: [0u8; 32],
            state_root: [0u8; 32],
            timestamp,
            difficulty: 1,
            nonce: 0,
            coinbase: Address::zero(),
        },
        transactions: Vec::new(),
    }
}

#[test]
fn test_chain_sync_request_response_flow() {
    let genesis = Block::genesis();
    let genesis_hash = genesis.hash();
    let block1 = make_block(1, genesis_hash, genesis.header.timestamp + 1);
    let block2 = make_block(2, block1.hash(), block1.header.timestamp + 1);

    let peer_manager = PeerManager::new(PeerConfig::default());
    let peer = PeerId::random();
    peer_manager.add_peer(peer, None);
    peer_manager.on_peer_connected(peer, false);

    let sync = ChainSync::new(SyncConfig::default(), genesis_hash, 0, genesis_hash);

    let start_requests = sync.start_sync(&peer_manager);
    assert_eq!(start_requests.len(), 1);
    assert!(matches!(start_requests[0].1, SyncRequest::GetStatus));

    let headers_request = sync
        .on_status_response(peer, 2, block2.hash(), genesis_hash, &peer_manager)
        .expect("expected header sync request");
    assert!(matches!(
        headers_request.1,
        SyncRequest::GetHeaders { .. }
    ));

    let follow_up_requests = sync.on_headers_response(
        peer,
        vec![block1.header.clone(), block2.header.clone()],
        &peer_manager,
    );
    assert!(!follow_up_requests.is_empty());
    assert!(follow_up_requests
        .iter()
        .any(|(_, req)| matches!(req, SyncRequest::GetBlocks { .. })));

    let next_requests = sync.on_blocks_response(peer, vec![block1.clone(), block2.clone()], &peer_manager);
    assert!(next_requests.is_empty() || next_requests.iter().all(|(_, req)| matches!(req, SyncRequest::GetBlocks { .. })));
    assert!(sync.has_blocks_to_import());

    let imported_1 = sync.next_block_to_import().expect("expected first block");
    let imported_2 = sync.next_block_to_import().expect("expected second block");
    assert_eq!(imported_1.height(), 1);
    assert_eq!(imported_2.height(), 2);
}

#[test]
fn test_fresh_node_sync_progress_to_remote_tip() {
    let genesis = Block::genesis();
    let genesis_hash = genesis.hash();
    let block1 = make_block(1, genesis_hash, genesis.header.timestamp + 1);
    let block2 = make_block(2, block1.hash(), block1.header.timestamp + 1);

    let peer_manager = PeerManager::new(PeerConfig::default());
    let peer = PeerId::random();
    peer_manager.add_peer(peer, None);
    peer_manager.on_peer_connected(peer, false);

    let sync = ChainSync::new(SyncConfig::default(), genesis_hash, 0, genesis_hash);
    assert_eq!(sync.progress(), 1.0);

    let _ = sync.start_sync(&peer_manager);
    let _ = sync.on_status_response(peer, 2, block2.hash(), genesis_hash, &peer_manager);
    let _ = sync.on_headers_response(peer, vec![block1.header.clone(), block2.header.clone()], &peer_manager);
    let _ = sync.on_blocks_response(peer, vec![block1.clone(), block2.clone()], &peer_manager);

    sync.update_local_state(1, block1.hash());
    assert!(sync.progress() < 1.0);

    sync.update_local_state(2, block2.hash());
    assert_eq!(sync.target_height(), 2);
    assert_eq!(sync.progress(), 1.0);
}
