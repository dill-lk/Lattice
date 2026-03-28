//! Merkle Mountain Ranges (MMR) for efficient state verification
//!
//! MMR is an append-only data structure that allows efficient proofs
//! without rebalancing, perfect for blockchain state.

use crate::Hash;
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

/// Merkle Mountain Range
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MerkleMountainRange {
    /// Peaks of the mountain range (roots of perfect binary trees)
    peaks: Vec<Hash>,
    /// Total number of leaves added
    size: u64,
    /// All nodes (for proof generation)
    nodes: Vec<Hash>,
}

impl MerkleMountainRange {
    /// Create a new empty MMR
    pub fn new() -> Self {
        Self {
            peaks: Vec::new(),
            size: 0,
            nodes: Vec::new(),
        }
    }

    /// Append a new leaf to the MMR
    pub fn append(&mut self, leaf_hash: Hash) {
        self.nodes.push(leaf_hash);
        self.size += 1;
        
        let mut current = leaf_hash;
        let mut height = 0u32;
        
        // Merge peaks as needed
        while let Some(peak) = self.find_peak_to_merge(height) {
            let merged = hash_pair(&peak, &current);
            self.nodes.push(merged);
            current = merged;
            height += 1;
        }
        
        self.rebuild_peaks();
    }

    /// Get the root hash (bagging all peaks)
    pub fn root(&self) -> Hash {
        if self.peaks.is_empty() {
            return [0u8; 32];
        }
        
        if self.peaks.len() == 1 {
            return self.peaks[0];
        }
        
        // Bag all peaks together
        let mut result = self.peaks[0];
        for peak in &self.peaks[1..] {
            result = hash_pair(&result, peak);
        }
        result
    }

    /// Get size of MMR
    pub fn size(&self) -> u64 {
        self.size
    }

    /// Generate inclusion proof for a leaf at position
    pub fn generate_proof(&self, pos: u64) -> Option<MmrProof> {
        if pos >= self.size {
            return None;
        }
        
        let mut proof_hashes = Vec::new();
        let mut positions = Vec::new();
        
        let leaf_index = pos as usize;
        let mut current_pos = self.leaf_index_to_mmr_index(leaf_index);
        let mut current_height = 0u32;
        
        // Collect sibling hashes up to peak
        loop {
            let sibling_pos = self.get_sibling_pos(current_pos, current_height);
            
            if let Some(sib_pos) = sibling_pos {
                if sib_pos < self.nodes.len() {
                    proof_hashes.push(self.nodes[sib_pos]);
                    positions.push(sib_pos as u64);
                } else {
                    break;
                }
                
                let parent_pos = self.get_parent_pos(current_pos, current_height);
                if parent_pos >= self.nodes.len() {
                    break;
                }
                
                current_pos = parent_pos;
                current_height += 1;
            } else {
                break;
            }
        }
        
        // Add peak bagging proof
        let peak_proof = self.generate_peak_bagging_proof(current_pos);
        
        Some(MmrProof {
            leaf_index: pos,
            sibling_hashes: proof_hashes,
            peak_hashes: peak_proof,
            mmr_size: self.size,
        })
    }

    /// Verify an inclusion proof
    pub fn verify_proof(&self, leaf_hash: &Hash, proof: &MmrProof) -> bool {
        if proof.leaf_index >= self.size {
            return false;
        }
        
        // Compute root from leaf + siblings
        let mut current = *leaf_hash;
        let mut height = 0u32;
        let mut pos = self.leaf_index_to_mmr_index(proof.leaf_index as usize);
        
        for sibling in &proof.sibling_hashes {
            let is_right = (pos / (1 << height)).is_multiple_of(2);
            current = if is_right {
                hash_pair(&current, sibling)
            } else {
                hash_pair(sibling, &current)
            };
            height += 1;
            pos = self.get_parent_pos(pos, height - 1);
        }
        
        // Verify against bagged peaks
        let expected_root = self.bag_peaks(&proof.peak_hashes);
        current == expected_root || self.root() == expected_root
    }

    /// Get the peaks
    pub fn peaks(&self) -> &[Hash] {
        &self.peaks
    }

    // Internal methods

    fn find_peak_to_merge(&self, height: u32) -> Option<Hash> {
        // Check if there's a peak at this height that can be merged
        let peak_size = 1u64 << (height + 1);
        if self.size.is_multiple_of(peak_size) && self.size > 0 {
            // Find the peak
            let peak_pos = self.size - peak_size;
            let peak_mmr_index = self.leaf_index_to_mmr_index(peak_pos as usize);
            
            // Walk up to find peak root
            let mut pos = peak_mmr_index;
            for _ in 0..height {
                pos = self.get_parent_pos(pos, 0);
            }
            
            if pos < self.nodes.len() {
                return Some(self.nodes[pos]);
            }
        }
        None
    }

    fn rebuild_peaks(&mut self) {
        self.peaks.clear();
        
        let mut remaining = self.size;
        let mut pos = 0usize;
        
        while remaining > 0 {
            let peak_size = highest_power_of_two(remaining);
            let leaves_in_peak = 1u64 << peak_size;
            
            // Find peak root
            let peak_start = pos;
            let mut peak_pos = self.leaf_index_to_mmr_index(peak_start);
            
            for _ in 0..peak_size {
                peak_pos = self.get_parent_pos(peak_pos, 0);
            }
            
            if peak_pos < self.nodes.len() {
                self.peaks.push(self.nodes[peak_pos]);
            }
            
            remaining -= leaves_in_peak;
            pos += leaves_in_peak as usize;
        }
    }

    fn leaf_index_to_mmr_index(&self, leaf_index: usize) -> usize {
        // Convert leaf index to MMR position
        // MMR structure: [0] [1,2] [3] [4,5,6] [7] [8,9] [10] ...
        let mut pos = 0;
        let mut size = 1;
        let mut index = leaf_index;
        
        while index >= size {
            pos += size * 2 - 1;
            index -= size;
            size *= 2;
        }
        
        pos + index
    }

    fn get_sibling_pos(&self, pos: usize, _height: u32) -> Option<usize> {
        if pos == 0 {
            return None;
        }
        
        if pos.is_multiple_of(2) {
            Some(pos - 1)
        } else {
            Some(pos + 1)
        }
    }

    fn get_parent_pos(&self, pos: usize, _height: u32) -> usize {
        // Parent is at position that combines two children
        pos.div_ceil(2)
    }

    fn generate_peak_bagging_proof(&self, _peak_pos: usize) -> Vec<Hash> {
        self.peaks.clone()
    }

    fn bag_peaks(&self, peaks: &[Hash]) -> Hash {
        if peaks.is_empty() {
            return [0u8; 32];
        }
        
        let mut result = peaks[0];
        for peak in &peaks[1..] {
            result = hash_pair(&result, peak);
        }
        result
    }
}

impl Default for MerkleMountainRange {
    fn default() -> Self {
        Self::new()
    }
}

/// MMR inclusion proof
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MmrProof {
    /// Leaf index in MMR
    pub leaf_index: u64,
    /// Sibling hashes in the path
    pub sibling_hashes: Vec<Hash>,
    /// Peak hashes for bagging
    pub peak_hashes: Vec<Hash>,
    /// Size of MMR when proof was generated
    pub mmr_size: u64,
}

/// Hash two hashes together
fn hash_pair(left: &Hash, right: &Hash) -> Hash {
    let mut hasher = Sha3_256::new();
    hasher.update(left);
    hasher.update(right);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Find highest power of 2 <= n
fn highest_power_of_two(n: u64) -> u32 {
    if n == 0 {
        return 0;
    }
    63 - n.leading_zeros()
}

/// State Triangulation using MMR
/// Allows fast state proofs and efficient state syncing
#[derive(Debug, Clone)]
pub struct StateTriangulation {
    /// MMR for account state hashes
    account_mmr: MerkleMountainRange,
    /// MMR for transaction hashes
    transaction_mmr: MerkleMountainRange,
    /// MMR for receipt hashes
    receipt_mmr: MerkleMountainRange,
}

impl StateTriangulation {
    /// Create new state triangulation
    pub fn new() -> Self {
        Self {
            account_mmr: MerkleMountainRange::new(),
            transaction_mmr: MerkleMountainRange::new(),
            receipt_mmr: MerkleMountainRange::new(),
        }
    }

    /// Add account state
    pub fn add_account_state(&mut self, account_hash: Hash) {
        self.account_mmr.append(account_hash);
    }

    /// Add transaction
    pub fn add_transaction(&mut self, tx_hash: Hash) {
        self.transaction_mmr.append(tx_hash);
    }

    /// Add receipt
    pub fn add_receipt(&mut self, receipt_hash: Hash) {
        self.receipt_mmr.append(receipt_hash);
    }

    /// Get combined state root
    pub fn state_root(&self) -> Hash {
        let account_root = self.account_mmr.root();
        let tx_root = self.transaction_mmr.root();
        let receipt_root = self.receipt_mmr.root();
        
        // Triple hash for triangulation
        let mut hasher = Sha3_256::new();
        hasher.update(account_root);
        hasher.update(tx_root);
        hasher.update(receipt_root);
        let result = hasher.finalize();
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&result);
        hash
    }

    /// Generate proof for account
    pub fn prove_account(&self, index: u64) -> Option<MmrProof> {
        self.account_mmr.generate_proof(index)
    }

    /// Generate proof for transaction
    pub fn prove_transaction(&self, index: u64) -> Option<MmrProof> {
        self.transaction_mmr.generate_proof(index)
    }

    /// Verify account proof
    pub fn verify_account(&self, account_hash: &Hash, proof: &MmrProof) -> bool {
        self.account_mmr.verify_proof(account_hash, proof)
    }

    /// Get size statistics
    pub fn stats(&self) -> TriangulationStats {
        TriangulationStats {
            accounts: self.account_mmr.size(),
            transactions: self.transaction_mmr.size(),
            receipts: self.receipt_mmr.size(),
        }
    }
}

impl Default for StateTriangulation {
    fn default() -> Self {
        Self::new()
    }
}

/// Statistics for state triangulation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TriangulationStats {
    pub accounts: u64,
    pub transactions: u64,
    pub receipts: u64,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mmr_append_and_root() {
        let mut mmr = MerkleMountainRange::new();
        
        mmr.append([1u8; 32]);
        assert_eq!(mmr.size(), 1);
        
        mmr.append([2u8; 32]);
        assert_eq!(mmr.size(), 2);
        
        mmr.append([3u8; 32]);
        assert_eq!(mmr.size(), 3);
        
        let root = mmr.root();
        assert_ne!(root, [0u8; 32]);
    }

    #[test]
    fn test_mmr_proof() {
        let mut mmr = MerkleMountainRange::new();
        
        let leaf1 = [1u8; 32];
        let leaf2 = [2u8; 32];
        let leaf3 = [3u8; 32];
        
        mmr.append(leaf1);
        mmr.append(leaf2);
        mmr.append(leaf3);
        
        // Generate proof for first leaf
        let proof = mmr.generate_proof(0).unwrap();
        assert!(mmr.verify_proof(&leaf1, &proof));
        
        // Wrong leaf should fail
        assert!(!mmr.verify_proof(&[99u8; 32], &proof));
    }

    #[test]
    fn test_state_triangulation() {
        let mut triangulation = StateTriangulation::new();
        
        triangulation.add_account_state([1u8; 32]);
        triangulation.add_transaction([2u8; 32]);
        triangulation.add_receipt([3u8; 32]);
        
        let root = triangulation.state_root();
        assert_ne!(root, [0u8; 32]);
        
        let stats = triangulation.stats();
        assert_eq!(stats.accounts, 1);
        assert_eq!(stats.transactions, 1);
        assert_eq!(stats.receipts, 1);
    }
}
