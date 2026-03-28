//! Integration tests for block validation

use lattice_core::{Block, BlockHeader, Transaction, TransactionKind, Address, Account, State, Network};
use lattice_crypto::{Keypair, sha3_256};
use lattice_consensus::{PoWConfig, verify_pow, hash_block_header};

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

#[test]
fn test_merkle_root_single_transaction() {
    let keypair = Keypair::generate();
    let from = Address::from_public_key(&keypair.public.to_vec());
    let to = Address::from_bytes([1u8; 20]);
    
    let tx = Transaction {
        kind: TransactionKind::Transfer,
        from,
        to,
        amount: 1000,
        fee: 100,
        nonce: 0,
        data: vec![],
        gas_limit: 21000,
        chain_id: Network::Mainnet.chain_id(),
        public_key: keypair.public.to_vec(),
        signature: vec![],
    };
    
    let root = Block::calculate_tx_root(&[tx.clone()]);
    
    // Merkle root of single item should be hash of that item
    let expected = sha3_256(&borsh::to_vec(&tx).unwrap());
    assert_eq!(root, expected);
}

#[test]
fn test_merkle_root_multiple_transactions() {
    let keypair = Keypair::generate();
    let from = Address::from_public_key(&keypair.public.to_vec());
    
    let tx1 = Transaction {
        kind: TransactionKind::Transfer,
        from: from.clone(),
        to: Address::from_bytes([1u8; 20]),
        amount: 1000,
        fee: 100,
        nonce: 0,
        data: vec![],
        gas_limit: 21000,
        chain_id: Network::Mainnet.chain_id(),
        public_key: keypair.public.to_vec(),
        signature: vec![],
    };
    
    let tx2 = Transaction {
        kind: TransactionKind::Transfer,
        from: from.clone(),
        to: Address::from_bytes([2u8; 20]),
        amount: 2000,
        fee: 100,
        nonce: 1,
        data: vec![],
        gas_limit: 21000,
        chain_id: Network::Mainnet.chain_id(),
        public_key: keypair.public.to_vec(),
        signature: vec![],
    };
    
    let root1 = Block::calculate_tx_root(&[tx1.clone(), tx2.clone()]);
    let root2 = Block::calculate_tx_root(&[tx2.clone(), tx1.clone()]);
    
    // Different order should produce different merkle roots
    assert_ne!(root1, root2);
    
    // Same transactions in same order should produce same root
    let root3 = Block::calculate_tx_root(&[tx1.clone(), tx2.clone()]);
    assert_eq!(root1, root3);
}

#[test]
fn test_block_with_transactions() {
    let genesis = Block::genesis();
    let keypair = Keypair::generate();
    let from = Address::from_public_key(&keypair.public.to_vec());
    
    let tx = Transaction {
        kind: TransactionKind::Transfer,
        from,
        to: Address::from_bytes([1u8; 20]),
        amount: 1000,
        fee: 100,
        nonce: 0,
        data: vec![],
        gas_limit: 21000,
        chain_id: Network::Mainnet.chain_id(),
        public_key: keypair.public.to_vec(),
        signature: vec![],
    };
    
    let mut block = Block {
        header: BlockHeader {
            version: 1,
            height: 1,
            prev_hash: genesis.hash(),
            tx_root: Block::calculate_tx_root(&[tx.clone()]),
            state_root: [0u8; 32],
            timestamp: 1234567890,
            difficulty: 1000,
            nonce: 0,
            coinbase: Address::from_bytes([0u8; 20]),
        },
        transactions: vec![tx],
    };
    
    // Verify merkle root is correct
    let calculated_root = Block::calculate_tx_root(&block.transactions);
    assert_eq!(block.header.tx_root, calculated_root);
}

#[test]
fn test_block_validation_basic() {
    let genesis = Block::genesis();
    
    // Genesis block should be valid
    assert_eq!(genesis.height(), 0);
    assert_eq!(genesis.header.prev_hash, [0u8; 32]);
}

#[test]
fn test_block_chain_validation() {
    let genesis = Block::genesis();
    let genesis_hash = genesis.hash();
    
    let block1 = Block {
        header: BlockHeader {
            version: 1,
            height: 1,
            prev_hash: genesis_hash,
            tx_root: [0u8; 32],
            state_root: [0u8; 32],
            timestamp: 1000,
            difficulty: 1000,
            nonce: 0,
            coinbase: Address::from_bytes([0u8; 20]),
        },
        transactions: vec![],
    };
    
    // Block 1 should reference genesis
    assert_eq!(block1.header.prev_hash, genesis_hash);
    assert_eq!(block1.height(), 1);
}

#[test]
fn test_pow_verification_light() {
    let config = PoWConfig::light();
    let coinbase = Address::from_bytes([1u8; 20]);
    
    let mut header = BlockHeader {
        version: 1,
        height: 0,
        prev_hash: [0u8; 32],
        tx_root: [0u8; 32],
        state_root: [0u8; 32],
        timestamp: 1234567890,
        difficulty: 1, // Very low difficulty for testing
        nonce: 0,
        coinbase,
    };
    
    // Try a few nonces to find one that works
    let mut found = false;
    for nonce in 0..1000 {
        header.nonce = nonce;
        if verify_pow(&header, &config).is_ok() {
            found = true;
            break;
        }
    }
    
    assert!(found, "Should find valid nonce within 1000 attempts with difficulty 1");
}

#[test]
fn test_state_transitions() {
    let mut state = State::new();
    let addr1 = Address::from_bytes([1u8; 20]);
    let addr2 = Address::from_bytes([2u8; 20]);
    
    // Initialize account with balance
    state.set_account(addr1.clone(), Account::with_balance(1000));
    
    // Transfer some balance
    state.transfer(&addr1, &addr2, 400).unwrap();
    
    assert_eq!(state.balance(&addr1), 600);
    assert_eq!(state.balance(&addr2), 400);
    
    // Check nonces
    assert_eq!(state.nonce(&addr1), 0);
    assert_eq!(state.nonce(&addr2), 0);
}

#[test]
fn test_state_root_changes() {
    let mut state1 = State::new();
    let mut state2 = State::new();
    
    let addr = Address::from_bytes([1u8; 20]);
    
    state1.set_account(addr.clone(), Account::with_balance(1000));
    state2.set_account(addr.clone(), Account::with_balance(2000));
    
    // Different balances should produce different state roots
    assert_ne!(state1.root(), state2.root());
}

#[test]
fn test_transaction_signing_and_verification() {
    let keypair = Keypair::generate();
    let from = Address::from_public_key(&keypair.public.to_vec());
    let to = Address::from_bytes([1u8; 20]);
    
    let mut tx = Transaction {
        kind: TransactionKind::Transfer,
        from: from.clone(),
        to,
        amount: 1000,
        fee: 100,
        nonce: 0,
        data: vec![],
        gas_limit: 21000,
        chain_id: Network::Mainnet.chain_id(),
        public_key: keypair.public.to_vec(),
        signature: vec![],
    };
    
    // Sign transaction
    let signing_bytes = tx.signing_bytes();
    let signature = keypair.sign(&signing_bytes);
    tx.signature = signature.to_vec();
    
    // Verify signature
    assert!(tx.is_signed());
    assert!(tx.verify_signature());
}

#[test]
fn test_transaction_gas_calculation() {
    let keypair = Keypair::generate();
    let from = Address::from_public_key(&keypair.public.to_vec());
    
    // Transfer transaction (no data)
    let tx1 = Transaction {
        kind: TransactionKind::Transfer,
        from: from.clone(),
        to: Address::from_bytes([1u8; 20]),
        amount: 1000,
        fee: 100,
        nonce: 0,
        data: vec![],
        gas_limit: 21000,
        chain_id: Network::Mainnet.chain_id(),
        public_key: keypair.public.to_vec(),
        signature: vec![],
    };
    
    // Transaction with data costs more
    let tx2 = Transaction {
        kind: TransactionKind::Transfer,
        from: from.clone(),
        to: Address::from_bytes([1u8; 20]),
        amount: 1000,
        fee: 100,
        nonce: 0,
        data: vec![0u8; 100], // 100 bytes of data
        gas_limit: 21000,
        chain_id: Network::Mainnet.chain_id(),
        public_key: keypair.public.to_vec(),
        signature: vec![],
    };
    
    let gas1 = tx1.gas_cost();
    let gas2 = tx2.gas_cost();
    
    assert!(gas2 > gas1, "Transaction with data should cost more gas");
}
