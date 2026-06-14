//! Protocol consistency integration tests.

use lattice_core::validation::{
    execute_block, validate_block, BlockValidationContext, TxValidationContext,
    validate_transaction,
};
use lattice_core::{Account, Address, Block, BlockHeader, Network, State, Transaction};
use lattice_crypto::Keypair;

#[test]
fn test_block_execution_matches_header_roots_and_balances() {
    let network = Network::Devnet;
    let genesis = lattice_core::genesis::create_genesis(&lattice_core::genesis::GenesisConfig::for_network(network))
        .unwrap();
    let genesis_block = genesis.block;
    let mut pre_state = genesis.state;

    let sender_keys = Keypair::generate();
    let sender = Address::from_public_key(&sender_keys.public.to_vec());
    let recipient = Address::from_bytes([0x11; 20]);
    let miner = Address::from_bytes([0x22; 20]);

    pre_state.set_account(sender.clone(), Account::with_balance(5_000_000_000));

    let mut tx = Transaction::transfer(
        sender.clone(),
        recipient.clone(),
        1_000_000_000,
        25_000,
        0,
        network.chain_id(),
    );
    tx.public_key = sender_keys.public.to_vec();
    tx.signature = sender_keys.sign(&tx.signing_bytes()).to_vec();

    let tx_ctx = TxValidationContext {
        state: &pre_state,
        chain_id: network.chain_id(),
        current_timestamp: genesis_block.header.timestamp + 1,
    };
    validate_transaction(&tx, &tx_ctx).unwrap();

    let candidate_txs = vec![tx.clone()];
    let mut expected_post_state = execute_block(
        &Block {
            header: BlockHeader {
                version: 1,
                height: 1,
                prev_hash: genesis_block.hash(),
                tx_root: Block::calculate_tx_root(&candidate_txs),
                state_root: [0u8; 32],
                timestamp: genesis_block.header.timestamp + 1,
                difficulty: 1,
                nonce: 0,
                coinbase: miner.clone(),
            },
            transactions: candidate_txs.clone(),
        },
        pre_state.clone(),
    )
    .unwrap();

    let expected_state_root = expected_post_state.root();

    let block = Block {
        header: BlockHeader {
            version: 1,
            height: 1,
            prev_hash: genesis_block.hash(),
            tx_root: Block::calculate_tx_root(&candidate_txs),
            state_root: expected_state_root,
            timestamp: genesis_block.header.timestamp + 1,
            difficulty: 1,
            nonce: 0,
            coinbase: miner.clone(),
        },
        transactions: candidate_txs,
    };

    let block_ctx = BlockValidationContext {
        parent_block: Some(&genesis_block),
        current_timestamp: genesis_block.header.timestamp + 1,
        state: &pre_state,
    };
    validate_block(&block, &block_ctx).unwrap();

    let actual_post_state = execute_block(&block, pre_state).unwrap();
    assert_eq!(actual_post_state.root(), expected_post_state.root());

    // sender: 5_000_000_000 - 1_000_000_000 - 25_000
    assert_eq!(actual_post_state.balance(&sender), 3_999_975_000);
    assert_eq!(actual_post_state.balance(&recipient), 1_000_000_000);
    assert_eq!(actual_post_state.nonce(&sender), 1);

    // miner: block reward + fee
    assert_eq!(
        actual_post_state.balance(&miner),
        lattice_core::tokenomics::BLOCK_REWARD + 25_000
    );
}

#[test]
fn test_transaction_signature_rejects_mutated_payload() {
    let keypair = Keypair::generate();
    let sender = Address::from_public_key(&keypair.public.to_vec());
    let recipient = Address::from_bytes([0x44; 20]);

    let mut tx = Transaction::transfer(sender, recipient, 10_000, 1000, 0, Network::Mainnet.chain_id());
    tx.public_key = keypair.public.to_vec();
    tx.signature = keypair.sign(&tx.signing_bytes()).to_vec();
    assert!(tx.verify_signature());

    tx.amount += 1;
    assert!(!tx.verify_signature());
}
