//! Integration tests for block validation

use lattice_core::{Block, Transaction, Address, Account, State};

#[test]
fn test_genesis_block_is_valid() {
    let genesis = Block::genesis();
    assert_eq!(genesis.height(), 0);
    assert!(genesis.transactions.is_empty());
    assert_eq!(genesis.header.prev_hash, [0u8; 32]);
}

#[test]
fn test_block_hash_changes_with_content() {
    let block1 = Block::genesis();
    let mut block2 = Block::genesis();
    block2.header.nonce = 1;
    
    assert_ne!(block1.hash(), block2.hash());
}

#[test]
fn test_merkle_root_empty() {
    let root = Block::calculate_tx_root(&[]);
    assert_eq!(root, [0u8; 32]);
}
