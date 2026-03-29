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
    /// All leaf hashes (stored as `nodes` for serialization compatibility)
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
        self.rebuild_peaks();
    }

    /// Get the root hash (bagging all peaks)
    pub fn root(&self) -> Hash {
        self.bag_peaks(&self.peaks)
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

        let leaf_index = pos as usize;
        let n = self.size as usize;

        let (peak_idx, tree_start, tree_size) = Self::find_peak_for_leaf(n, leaf_index)?;

        let mut sibling_hashes = Vec::new();
        Self::collect_proof(
            &self.nodes[tree_start..tree_start + tree_size],
            leaf_index - tree_start,
            &mut sibling_hashes,
        );

        Some(MmrProof {
            leaf_index: pos,
            sibling_hashes,
            peak_hashes: self.peaks.clone(),
            mmr_size: self.size,
        })
    }

    /// Verify an inclusion proof
    pub fn verify_proof(&self, leaf_hash: &Hash, proof: &MmrProof) -> bool {
        if proof.leaf_index >= self.size {
            return false;
        }

        let leaf_index = proof.leaf_index as usize;
        let n = self.size as usize;

        let (peak_idx, tree_start, tree_size) =
            match Self::find_peak_for_leaf(n, leaf_index) {
                Some(v) => v,
                None => return false,
            };

        let local_index = leaf_index - tree_start;
        let reconstructed_peak =
            Self::verify_path(leaf_hash, local_index, tree_size, &proof.sibling_hashes);

        if peak_idx >= proof.peak_hashes.len()
            || reconstructed_peak != proof.peak_hashes[peak_idx]
        {
            return false;
        }

        // The peaks in the proof must also bag to our current root.
        self.bag_peaks(&proof.peak_hashes) == self.root()
    }

    /// Get the peaks
    pub fn peaks(&self) -> &[Hash] {
        &self.peaks
    }

    // Internal methods

    /// Rebuild the peaks list from the stored leaves.
    ///
    /// Peaks are the roots of maximal perfect binary trees whose sizes sum to
    /// `self.size`.  The sizes are determined by the binary representation of
    /// `self.size`.
    fn rebuild_peaks(&mut self) {
        self.peaks.clear();
        let n = self.nodes.len();
        let mut remaining = n;
        let mut start = 0;

        while remaining > 0 {
            // Largest power-of-two ≤ remaining
            let height = 63 - (remaining as u64).leading_zeros();
            let tree_size = 1usize << height;
            self.peaks
                .push(Self::tree_root(&self.nodes[start..start + tree_size]));
            start += tree_size;
            remaining -= tree_size;
        }
    }

    /// Compute the Merkle root of a slice of leaf hashes.
    fn tree_root(leaves: &[Hash]) -> Hash {
        match leaves.len() {
            0 => [0u8; 32],
            1 => leaves[0],
            _ => {
                let mid = leaves.len() / 2;
                hash_pair(
                    &Self::tree_root(&leaves[..mid]),
                    &Self::tree_root(&leaves[mid..]),
                )
            }
        }
    }

    /// Collect sibling hashes for a proof (bottom-up order: leaf-level first).
    fn collect_proof(leaves: &[Hash], index: usize, proof: &mut Vec<Hash>) {
        if leaves.len() <= 1 {
            return;
        }
        let mid = leaves.len() / 2;
        if index < mid {
            Self::collect_proof(&leaves[..mid], index, proof);
            proof.push(Self::tree_root(&leaves[mid..]));
        } else {
            Self::collect_proof(&leaves[mid..], index - mid, proof);
            proof.push(Self::tree_root(&leaves[..mid]));
        }
    }

    /// Reconstruct the peak hash from a leaf and its sibling proof path.
    fn verify_path(
        leaf_hash: &Hash,
        mut index: usize,
        mut tree_size: usize,
        proof: &[Hash],
    ) -> Hash {
        let mut current = *leaf_hash;
        for sibling in proof {
            tree_size /= 2;
            if index < tree_size {
                current = hash_pair(&current, sibling);
            } else {
                index -= tree_size;
                current = hash_pair(sibling, &current);
            }
        }
        current
    }

    /// Return (peak_index, tree_start_leaf_index, tree_size) for a given leaf.
    fn find_peak_for_leaf(n: usize, leaf_index: usize) -> Option<(usize, usize, usize)> {
        let mut remaining = n;
        let mut start = 0;
        let mut peak_idx = 0;

        while remaining > 0 {
            let height = 63 - (remaining as u64).leading_zeros();
            let tree_size = 1usize << height;
            if leaf_index < start + tree_size {
                return Some((peak_idx, start, tree_size));
            }
            peak_idx += 1;
            start += tree_size;
            remaining -= tree_size;
        }
        None
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
