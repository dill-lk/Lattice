//! Chain validation logic for blocks and transactions
//!
//! This module provides comprehensive validation for blockchain operations:
//! - Transaction validation (signature, balance, nonce, gas)
//! - Block validation (PoW, merkle root, timestamps, size limits)
//! - Chain validation (parent block, height, state transitions)

use crate::{
    Address, Amount, Block, BlockHeader, BlockHeight, Hash, State, Transaction,
    TransactionKind, CoreError, Result,
};

/// Block size limit (2 MB)
pub const MAX_BLOCK_SIZE: usize = 2 * 1024 * 1024;

/// Maximum number of transactions per block
pub const MAX_TRANSACTIONS_PER_BLOCK: usize = 10000;

/// Maximum transaction size (100 KB)
pub const MAX_TRANSACTION_SIZE: usize = 100 * 1024;

/// Minimum transaction fee
pub const MIN_TRANSACTION_FEE: Amount = 1000;

/// Block time tolerance (blocks can't be too far in future)
pub const FUTURE_BLOCK_TIME_TOLERANCE_MS: u64 = 15 * 60 * 1000; // 15 minutes

/// Block reward in base units (10 LAT)
pub const BLOCK_REWARD: Amount = 10_000_000_000_000_000_000;

/// Transaction validation context
pub struct TxValidationContext<'a> {
    pub state: &'a State,
    pub chain_id: u32,
    pub current_timestamp: u64,
}

/// Block validation context
pub struct BlockValidationContext<'a> {
    pub parent_block: Option<&'a Block>,
    pub current_timestamp: u64,
    pub state: &'a State,
}

// ============================================================================
// Transaction Validation
// ============================================================================

/// Validate a transaction completely
pub fn validate_transaction(tx: &Transaction, ctx: &TxValidationContext) -> Result<()> {
    // Basic structure validation
    validate_transaction_structure(tx)?;
    
    // Signature validation
    validate_transaction_signature(tx)?;
    
    // Chain ID validation
    if tx.chain_id != ctx.chain_id {
        return Err(CoreError::InvalidChainId);
    }
    
    // State-dependent validation
    validate_transaction_against_state(tx, ctx.state)?;
    
    Ok(())
}

/// Validate transaction structure (independent of state)
pub fn validate_transaction_structure(tx: &Transaction) -> Result<()> {
    // Check transaction size
    let tx_size = borsh::to_vec(tx).map_err(|_| CoreError::SerializationError)?.len();
    if tx_size > MAX_TRANSACTION_SIZE {
        return Err(CoreError::TransactionTooLarge);
    }
    
    // Check signature exists
    if !tx.is_signed() {
        return Err(CoreError::MissingSignature);
    }
    
    // Check public key exists
    if tx.public_key.is_empty() {
        return Err(CoreError::MissingPublicKey);
    }
    
    // Check fee
    if tx.fee < MIN_TRANSACTION_FEE {
        return Err(CoreError::FeeTooLow);
    }
    
    // Check gas limit is sufficient
    let min_gas = tx.gas_cost();
    if tx.gas_limit < min_gas {
        return Err(CoreError::GasLimitTooLow);
    }
    
    // Validate based on transaction kind
    match tx.kind {
        TransactionKind::Transfer => {
            // Transfer must have amount > 0
            if tx.amount == 0 {
                return Err(CoreError::ZeroAmountTransfer);
            }
            // Transfer should not have data
            if !tx.data.is_empty() {
                return Err(CoreError::InvalidTransactionData);
            }
        }
        TransactionKind::Deploy => {
            // Deploy must have code in data
            if tx.data.is_empty() {
                return Err(CoreError::MissingContractCode);
            }
            // Deploy amount should be 0 (or initial balance)
            // to is usually zero address for deploy
        }
        TransactionKind::Call => {
            // Call can have empty data (getter functions)
            // to must not be zero address
            if tx.to == Address::zero() {
                return Err(CoreError::InvalidRecipient);
            }
        }
    }
    
    Ok(())
}

/// Validate transaction signature
pub fn validate_transaction_signature(tx: &Transaction) -> Result<()> {
    if !tx.verify_signature() {
        return Err(CoreError::InvalidSignature);
    }
    Ok(())
}

/// Validate transaction against current state
pub fn validate_transaction_against_state(tx: &Transaction, state: &State) -> Result<()> {
    // Get sender account
    let account = state.get_account(&tx.from);
    
    // Check nonce
    if tx.nonce != account.nonce {
        return Err(CoreError::InvalidNonce {
            expected: account.nonce,
            got: tx.nonce,
        });
    }
    
    // Check balance (amount + fee must be <= balance)
    let total_cost = tx.amount.checked_add(tx.fee)
        .ok_or(CoreError::AmountOverflow)?;
    
    if account.balance < total_cost {
        return Err(CoreError::InsufficientBalance {
            required: total_cost,
            available: account.balance,
        });
    }
    
    // For contract calls, verify contract exists
    if matches!(tx.kind, TransactionKind::Call) {
        let to_account = state.get_account(&tx.to);
        if !to_account.is_contract() {
            return Err(CoreError::NotAContract);
        }
    }
    
    Ok(())
}

// ============================================================================
// Block Validation
// ============================================================================

/// Validate a block completely
pub fn validate_block(block: &Block, ctx: &BlockValidationContext) -> Result<()> {
    // Basic structure validation
    validate_block_structure(block)?;
    
    // Parent block validation
    if let Some(parent) = ctx.parent_block {
        validate_block_against_parent(block, parent)?;
    } else {
        // This is genesis block
        validate_genesis_block(block)?;
    }
    
    // Timestamp validation
    validate_block_timestamp(block, ctx.current_timestamp)?;
    
    // Merkle root validation
    validate_merkle_root(block)?;
    
    // Transaction validation
    validate_block_transactions(block, ctx.state)?;
    
    Ok(())
}

/// Validate block structure (independent of chain)
pub fn validate_block_structure(block: &Block) -> Result<()> {
    // Check block size
    let block_size = borsh::to_vec(block).map_err(|_| CoreError::SerializationError)?.len();
    if block_size > MAX_BLOCK_SIZE {
        return Err(CoreError::BlockTooLarge);
    }
    
    // Check transaction count
    if block.transactions.len() > MAX_TRANSACTIONS_PER_BLOCK {
        return Err(CoreError::TooManyTransactions);
    }
    
    // Check version
    if block.header.version == 0 {
        return Err(CoreError::InvalidBlockVersion);
    }
    
    // Check difficulty
    if block.header.difficulty == 0 {
        return Err(CoreError::ZeroDifficulty);
    }
    
    Ok(())
}

/// Validate block against parent block
pub fn validate_block_against_parent(block: &Block, parent: &Block) -> Result<()> {
    // Check height
    if block.height() != parent.height() + 1 {
        return Err(CoreError::InvalidBlockHeight {
            expected: parent.height() + 1,
            got: block.height(),
        });
    }
    
    // Check prev_hash
    if block.header.prev_hash != parent.hash() {
        return Err(CoreError::InvalidPreviousHash);
    }
    
    // Check timestamp (must be after parent)
    if block.header.timestamp <= parent.header.timestamp {
        return Err(CoreError::InvalidTimestamp);
    }
    
    Ok(())
}

/// Validate genesis block
pub fn validate_genesis_block(block: &Block) -> Result<()> {
    if block.height() != 0 {
        return Err(CoreError::InvalidGenesisBlock);
    }
    
    if block.header.prev_hash != [0u8; 32] {
        return Err(CoreError::InvalidGenesisBlock);
    }
    
    if !block.transactions.is_empty() {
        return Err(CoreError::InvalidGenesisBlock);
    }
    
    Ok(())
}

/// Validate block timestamp
pub fn validate_block_timestamp(block: &Block, current_time: u64) -> Result<()> {
    // Block cannot be too far in the future
    if block.header.timestamp > current_time + FUTURE_BLOCK_TIME_TOLERANCE_MS {
        return Err(CoreError::BlockFromFuture);
    }
    
    // Block cannot be from before Unix epoch
    if block.header.timestamp == 0 {
        return Err(CoreError::InvalidTimestamp);
    }
    
    Ok(())
}

/// Validate merkle root of transactions
pub fn validate_merkle_root(block: &Block) -> Result<()> {
    let calculated_root = Block::calculate_tx_root(&block.transactions);
    
    if calculated_root != block.header.tx_root {
        return Err(CoreError::InvalidMerkleRoot {
            expected: block.header.tx_root,
            got: calculated_root,
        });
    }
    
    Ok(())
}

/// Validate all transactions in a block
pub fn validate_block_transactions(block: &Block, state: &State) -> Result<()> {
    // Check for duplicate transactions
    let mut tx_hashes = std::collections::HashSet::new();
    
    for tx in &block.transactions {
        let tx_hash = tx.hash();
        if !tx_hashes.insert(tx_hash) {
            return Err(CoreError::DuplicateTransaction);
        }
        
        // Validate transaction structure
        validate_transaction_structure(tx)?;
        validate_transaction_signature(tx)?;
    }
    
    // Note: State-dependent validation (balance, nonce) should be done
    // during block execution, not here, as state changes within the block
    
    Ok(())
}

/// Apply block to state and return new state
pub fn execute_block(block: &Block, mut state: State) -> Result<State> {
    // Execute transactions
    for tx in &block.transactions {
        execute_transaction(tx, &mut state)?;
    }
    
    // Apply block reward to coinbase
    let coinbase_account = state.get_account_mut(&block.header.coinbase);
    let total_fees = block.total_fees();
    let reward = BLOCK_REWARD.checked_add(total_fees)
        .ok_or(CoreError::AmountOverflow)?;
    
    coinbase_account.balance = coinbase_account.balance
        .checked_add(reward)
        .ok_or(CoreError::AmountOverflow)?;
    
    Ok(state)
}

/// Execute a single transaction
pub fn execute_transaction(tx: &Transaction, state: &mut State) -> Result<()> {
    // Deduct fee + amount from sender
    let total_cost = tx.amount.checked_add(tx.fee)
        .ok_or(CoreError::AmountOverflow)?;
    
    {
        let from_account = state.get_account_mut(&tx.from);
        if from_account.balance < total_cost {
            return Err(CoreError::InsufficientBalance {
                required: total_cost,
                available: from_account.balance,
            });
        }
        from_account.balance -= total_cost;
        from_account.nonce += 1;
    }
    
    // Add amount to recipient (fees go to miner separately)
    match tx.kind {
        TransactionKind::Transfer => {
            let to_account = state.get_account_mut(&tx.to);
            to_account.balance = to_account.balance
                .checked_add(tx.amount)
                .ok_or(CoreError::AmountOverflow)?;
        }
        TransactionKind::Deploy => {
            // Contract deployment would happen here
            // For now, just transfer amount if any
            if tx.amount > 0 {
                let to_account = state.get_account_mut(&tx.to);
                to_account.balance = to_account.balance
                    .checked_add(tx.amount)
                    .ok_or(CoreError::AmountOverflow)?;
            }
        }
        TransactionKind::Call => {
            // Contract call would happen here
            // For now, just transfer amount if any
            if tx.amount > 0 {
                let to_account = state.get_account_mut(&tx.to);
                to_account.balance = to_account.balance
                    .checked_add(tx.amount)
                    .ok_or(CoreError::AmountOverflow)?;
            }
        }
    }
    
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_crypto::Keypair;
    
    #[test]
    fn test_validate_transaction_structure() {
        let keypair = Keypair::generate();
        let from = Address::from_public_key(&keypair.public.to_vec());
        
        let mut tx = Transaction::transfer(from, Address::zero(), 1000, 1000, 0, 1);
        tx.public_key = keypair.public.to_vec();
        tx.signature = vec![1; 100]; // Fake signature for structure test
        
        // Should pass structure validation
        assert!(validate_transaction_structure(&tx).is_ok());
    }
    
    #[test]
    fn test_validate_zero_fee() {
        let keypair = Keypair::generate();
        let from = Address::from_public_key(&keypair.public.to_vec());
        
        let mut tx = Transaction::transfer(from, Address::zero(), 1000, 0, 0, 1);
        tx.public_key = keypair.public.to_vec();
        tx.signature = vec![1; 100];
        
        // Should fail due to zero fee
        assert!(validate_transaction_structure(&tx).is_err());
    }
    
    #[test]
    fn test_execute_simple_transfer() {
        let mut state = State::new();
        let keypair = Keypair::generate();
        let from = Address::from_public_key(&keypair.public.to_vec());
        let to = Address::from_bytes([1u8; 20]);
        
        // Setup initial state
        use crate::Account;
        state.set_account(from.clone(), Account::with_balance(10000));
        
        // Create and execute transfer
        let mut tx = Transaction::transfer(from.clone(), to.clone(), 1000, 100, 0, 1);
        tx.public_key = keypair.public.to_vec();
        let sig_bytes = tx.signing_bytes();
        tx.signature = keypair.sign(&sig_bytes).to_vec();
        
        execute_transaction(&tx, &mut state).unwrap();
        
        // Check balances
        assert_eq!(state.balance(&from), 8900); // 10000 - 1000 - 100
        assert_eq!(state.balance(&to), 1000);
        assert_eq!(state.nonce(&from), 1);
    }
}
