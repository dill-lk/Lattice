//! Block types for Lattice blockchain

use crate::{Address, Amount, BlockHeight, Hash, Timestamp, Transaction};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

/// Block header containing metadata
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct BlockHeader {
    /// Block version
    pub version: u32,
    /// Height of this block
    pub height: BlockHeight,
    /// Hash of the previous block
    pub prev_hash: Hash,
    /// Merkle root of transactions
    pub tx_root: Hash,
    /// State root after applying transactions
    pub state_root: Hash,
    /// Block timestamp (ms since Unix epoch)
    pub timestamp: Timestamp,
    /// Difficulty target
    pub difficulty: u64,
    /// Nonce for PoW
    pub nonce: u64,
    /// Miner's address for block reward
    pub coinbase: Address,
}

impl BlockHeader {
    /// Calculate the hash of this header
    pub fn hash(&self) -> Hash {
        let bytes = borsh::to_vec(self).expect("serialization cannot fail");
        let digest = Sha3_256::digest(&bytes);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&digest);
        hash
    }

    /// Check if the hash meets the difficulty target
    pub fn meets_difficulty(&self) -> bool {
        let hash = self.hash();
        let target = difficulty_to_target(self.difficulty);
        hash_to_u256(&hash) <= target
    }
}

/// A complete block with header and transactions
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Block {
    /// Block header
    pub header: BlockHeader,
    /// Transactions in this block
    pub transactions: Vec<Transaction>,
}

impl Block {
    /// Create a new block
    pub fn new(header: BlockHeader, transactions: Vec<Transaction>) -> Self {
        Self { header, transactions }
    }

    /// Get the block hash
    pub fn hash(&self) -> Hash {
        self.header.hash()
    }

    /// Get block height
    pub fn height(&self) -> BlockHeight {
        self.header.height
    }

    /// Calculate the merkle root of transactions
    pub fn calculate_tx_root(transactions: &[Transaction]) -> Hash {
        if transactions.is_empty() {
            return [0u8; 32];
        }

        let mut hashes: Vec<Hash> = transactions
            .iter()
            .map(|tx| tx.hash())
            .collect();

        while hashes.len() > 1 {
            let mut next_level = Vec::new();
            for chunk in hashes.chunks(2) {
                let mut hasher = Sha3_256::new();
                hasher.update(chunk[0]);
                if chunk.len() > 1 {
                    hasher.update(chunk[1]);
                } else {
                    hasher.update(chunk[0]); // Duplicate last if odd
                }
                let mut hash = [0u8; 32];
                hash.copy_from_slice(&hasher.finalize());
                next_level.push(hash);
            }
            hashes = next_level;
        }

        hashes[0]
    }

    /// Calculate total fees in this block
    pub fn total_fees(&self) -> Amount {
        self.transactions.iter().map(|tx| tx.fee).sum()
    }

    /// Get the genesis block (basic version without allocations)
    /// For production use, prefer `lattice_core::genesis::create_genesis()` which includes
    /// the founder's initial allocation.
    pub fn genesis() -> Self {
        let header = BlockHeader {
            version: 1,
            height: 0,
            prev_hash: [0u8; 32],
            tx_root: [0u8; 32],
            state_root: [0u8; 32],
            timestamp: 1704067200000, // 2024-01-01 00:00:00 UTC
            difficulty: 1,
            nonce: 0,
            coinbase: Address::zero(),
        };
        
        Self {
            header,
            transactions: vec![],
        }
    }
}

/// Convert difficulty to target (higher difficulty = lower target)
fn difficulty_to_target(difficulty: u64) -> [u8; 32] {
    // Simple target = MAX_TARGET / difficulty
    let max_target = [0xFF; 32];
    let mut result = [0u8; 32];
    
    if difficulty == 0 {
        return max_target;
    }

    // Simplified calculation for demonstration
    let leading_zeros = (difficulty as f64).log2() as usize / 8;
    for byte in result.iter_mut().skip(leading_zeros) {
        *byte = 0xFF / (difficulty as u8).max(1);
    }
    
    result
}

/// Convert hash to comparable value
fn hash_to_u256(hash: &Hash) -> [u8; 32] {
    *hash
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_genesis_block() {
        let genesis = Block::genesis();
        assert_eq!(genesis.height(), 0);
        assert!(genesis.transactions.is_empty());
    }

    #[test]
    fn test_block_hash_deterministic() {
        let block = Block::genesis();
        let hash1 = block.hash();
        let hash2 = block.hash();
        assert_eq!(hash1, hash2);
    }
}
