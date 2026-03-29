//! Merkle tree and proof structures for efficient verification

use crate::Hash;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

/// Position in merkle tree
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Position {
    Left,
    Right,
}

/// A single node in a merkle proof path
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct MerkleNode {
    /// Hash of the sibling node
    pub hash: Hash,
    /// Position of this node (left or right)
    pub is_left: bool,
}

/// Merkle proof for inclusion
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct MerkleProof {
    /// Path from leaf to root
    pub path: Vec<MerkleNode>,
    /// Index of the leaf in the tree
    pub leaf_index: usize,
    /// Total number of leaves
    pub total_leaves: usize,
}

impl MerkleProof {
    /// Create a new merkle proof
    pub fn new(path: Vec<MerkleNode>, leaf_index: usize, total_leaves: usize) -> Self {
        Self {
            path,
            leaf_index,
            total_leaves,
        }
    }

    /// Verify that a leaf hash is included in the merkle tree with given root
    pub fn verify(&self, leaf_hash: &Hash, root: &Hash) -> bool {
        let computed_root = self.compute_root(leaf_hash);
        &computed_root == root
    }

    /// Compute the root hash from leaf and proof path
    pub fn compute_root(&self, leaf_hash: &Hash) -> Hash {
        let mut current_hash = *leaf_hash;

        for node in &self.path {
            current_hash = if node.is_left {
                hash_pair(&node.hash, &current_hash)
            } else {
                hash_pair(&current_hash, &node.hash)
            };
        }

        current_hash
    }

    /// Get the depth of the proof (height of tree)
    pub fn depth(&self) -> usize {
        self.path.len()
    }
}

/// Merkle tree for efficient proof generation
#[derive(Debug, Clone)]
pub struct MerkleTree {
    /// All nodes in the tree (level by level)
    nodes: Vec<Vec<Hash>>,
    /// Leaf hashes
    leaves: Vec<Hash>,
}

impl MerkleTree {
    /// Build a merkle tree from leaf hashes
    pub fn new(leaves: Vec<Hash>) -> Self {
        if leaves.is_empty() {
            return Self {
                nodes: vec![vec![[0u8; 32]]],
                leaves: vec![],
            };
        }

        let mut nodes = vec![leaves.clone()];
        let mut current_level = leaves.clone();

        // Build tree bottom-up, always running at least one hash step so
        // a single-leaf tree produces root = hash_pair(leaf, leaf).
        loop {
            let mut next_level = Vec::new();

            for chunk in current_level.chunks(2) {
                let hash = if chunk.len() == 2 {
                    hash_pair(&chunk[0], &chunk[1])
                } else {
                    // Odd number - duplicate last element
                    hash_pair(&chunk[0], &chunk[0])
                };
                next_level.push(hash);
            }

            nodes.push(next_level.clone());
            current_level = next_level;
            if current_level.len() <= 1 {
                break;
            }
        }

        Self { nodes, leaves }
    }

    /// Get the root hash
    pub fn root(&self) -> Hash {
        self.nodes.last().and_then(|level| level.first()).copied().unwrap_or([0u8; 32])
    }

    /// Get the number of leaves
    pub fn leaf_count(&self) -> usize {
        self.leaves.len()
    }

    /// Generate a proof for a specific leaf index
    pub fn generate_proof(&self, leaf_index: usize) -> Option<MerkleProof> {
        if leaf_index >= self.leaves.len() {
            return None;
        }

        let mut path = Vec::new();
        let mut current_index = leaf_index;

        // Traverse from leaf to root
        for level_index in 0..self.nodes.len() - 1 {
            let level = &self.nodes[level_index];
            let sibling_index = if current_index % 2 == 0 {
                current_index + 1
            } else {
                current_index - 1
            };

            // Get sibling hash
            let sibling_hash = if sibling_index < level.len() {
                level[sibling_index]
            } else {
                // Duplicate if no sibling
                level[current_index]
            };

            let is_left = current_index % 2 == 1;
            path.push(MerkleNode {
                hash: sibling_hash,
                is_left,
            });

            current_index /= 2;
        }

        Some(MerkleProof::new(path, leaf_index, self.leaves.len()))
    }

    /// Verify a proof against this tree's root
    pub fn verify_proof(&self, proof: &MerkleProof, leaf_hash: &Hash) -> bool {
        proof.verify(leaf_hash, &self.root())
    }
}

/// Hash two hashes together (merkle node combination)
pub fn hash_pair(left: &Hash, right: &Hash) -> Hash {
    let mut hasher = Sha3_256::new();
    hasher.update(left);
    hasher.update(right);
    let result = hasher.finalize();
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&result);
    hash
}

/// Compute merkle root from a list of hashes
pub fn compute_merkle_root(hashes: &[Hash]) -> Hash {
    if hashes.is_empty() {
        return [0u8; 32];
    }

    MerkleTree::new(hashes.to_vec()).root()
}

/// Sparse Merkle Tree for efficient state storage
#[derive(Debug, Clone)]
pub struct SparseMerkleTree {
    /// Depth of the tree (256 for 32-byte keys)
    depth: usize,
    /// Default hash at each level
    default_hashes: Vec<Hash>,
    /// Stored nodes (key -> hash)
    nodes: std::collections::HashMap<Vec<u8>, Hash>,
}

impl SparseMerkleTree {
    /// Create a new sparse merkle tree with given depth
    pub fn new(depth: usize) -> Self {
        // Compute default hashes for empty subtrees at each level
        let mut default_hashes = vec![[0u8; 32]; depth + 1];
        for i in (0..depth).rev() {
            default_hashes[i] = hash_pair(&default_hashes[i + 1], &default_hashes[i + 1]);
        }

        Self {
            depth,
            default_hashes,
            nodes: std::collections::HashMap::new(),
        }
    }

    /// Get the root hash
    pub fn root(&self) -> Hash {
        self.get_node(&[], 0)
    }

    /// Update a leaf value
    pub fn update(&mut self, key: &[u8; 32], value: &Hash) {
        self.update_leaf(&key[..], value, 0);
    }

    /// Get a leaf value
    pub fn get(&self, key: &[u8; 32]) -> Hash {
        self.get_leaf(&key[..], 0)
    }

    /// Generate a proof for a key
    pub fn generate_proof(&self, key: &[u8; 32]) -> Vec<Hash> {
        let mut proof = Vec::new();

        // Collect siblings from leaf level up to root (bottom-up order).
        // proof[0] = sibling at leaf level (depth self.depth),
        // proof[depth-1] = sibling one level below root (depth 1).
        for depth in (0..self.depth).rev() {
            let path_to_node = Self::key_to_path(key, depth + 1);
            let sibling_path = Self::sibling_path(&path_to_node);
            let sibling_hash = self.get_node(&sibling_path, depth + 1);
            proof.push(sibling_hash);
        }

        proof
    }

    /// Verify a proof
    pub fn verify_proof(&self, key: &[u8; 32], value: &Hash, proof: &[Hash]) -> bool {
        let mut current_hash = *value;

        // Traverse from leaf to root (bottom-up), matching proof order.
        for (i, sibling_hash) in proof.iter().enumerate() {
            let bit_pos = self.depth - 1 - i;
            let bit = (key[bit_pos / 8] >> (7 - (bit_pos % 8))) & 1;
            current_hash = if bit == 0 {
                // current node is the left child
                hash_pair(&current_hash, sibling_hash)
            } else {
                // current node is the right child
                hash_pair(sibling_hash, &current_hash)
            };
        }

        current_hash == self.root()
    }

    // Internal methods

    fn get_node(&self, path: &[bool], depth: usize) -> Hash {
        let key = Self::path_to_key(path);
        self.nodes.get(&key).copied().unwrap_or(self.default_hashes[depth])
    }

    fn set_node(&mut self, path: &[bool], hash: Hash) {
        let key = Self::path_to_key(path);
        self.nodes.insert(key, hash);
    }

    fn update_leaf(&mut self, key: &[u8], value: &Hash, depth: usize) {
        if depth == self.depth {
            self.set_node(&Self::key_to_path(key, self.depth), *value);
            return;
        }

        let bit = (key[depth / 8] >> (7 - (depth % 8))) & 1 == 1;
        let mut path = Self::key_to_path(key, depth);

        // Recurse down
        path.push(bit);
        self.update_leaf(key, value, depth + 1);

        // Update this node
        path.pop();
        let mut left_path = path.clone();
        left_path.push(false);
        let mut right_path = path.clone();
        right_path.push(true);

        let left_hash = self.get_node(&left_path, depth + 1);
        let right_hash = self.get_node(&right_path, depth + 1);
        let new_hash = hash_pair(&left_hash, &right_hash);

        self.set_node(&path, new_hash);
    }

    fn get_leaf(&self, key: &[u8], depth: usize) -> Hash {
        if depth == self.depth {
            return self.get_node(&Self::key_to_path(key, self.depth), depth);
        }

        let bit = (key[depth / 8] >> (7 - (depth % 8))) & 1 == 1;
        let mut path = Self::key_to_path(key, depth);
        path.push(bit);

        self.get_leaf(key, depth + 1)
    }

    fn path_to_key(path: &[bool]) -> Vec<u8> {
        path.iter().map(|&b| if b { 1u8 } else { 0u8 }).collect()
    }

    fn key_to_path(key: &[u8], depth: usize) -> Vec<bool> {
        (0..depth).map(|i| (key[i / 8] >> (7 - (i % 8))) & 1 == 1).collect()
    }

    fn sibling_path(path: &[bool]) -> Vec<bool> {
        let mut sibling = path.to_vec();
        if let Some(last) = sibling.last_mut() {
            *last = !*last;
        }
        sibling
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_merkle_tree_single_leaf() {
        let leaf = [1u8; 32];
        let tree = MerkleTree::new(vec![leaf]);

        assert_eq!(tree.root(), hash_pair(&leaf, &leaf));
        assert_eq!(tree.leaf_count(), 1);
    }

    #[test]
    fn test_merkle_tree_multiple_leaves() {
        let leaves = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        let tree = MerkleTree::new(leaves.clone());

        // Root should be deterministic
        let root1 = tree.root();
        let tree2 = MerkleTree::new(leaves);
        assert_eq!(root1, tree2.root());
    }

    #[test]
    fn test_merkle_proof_generation_and_verification() {
        let leaves = vec![[1u8; 32], [2u8; 32], [3u8; 32], [4u8; 32]];
        let tree = MerkleTree::new(leaves.clone());

        for (i, leaf) in leaves.iter().enumerate() {
            let proof = tree.generate_proof(i).unwrap();
            assert!(proof.verify(leaf, &tree.root()));
            assert!(tree.verify_proof(&proof, leaf));
        }
    }

    #[test]
    fn test_merkle_proof_invalid() {
        let leaves = vec![[1u8; 32], [2u8; 32], [3u8; 32]];
        let tree = MerkleTree::new(leaves);

        let proof = tree.generate_proof(0).unwrap();
        let wrong_leaf = [99u8; 32];

        assert!(!proof.verify(&wrong_leaf, &tree.root()));
    }

    #[test]
    fn test_sparse_merkle_tree() {
        let mut tree = SparseMerkleTree::new(8);

        let key1 = [1u8; 32];
        let key2 = [2u8; 32];
        let value1 = [10u8; 32];
        let value2 = [20u8; 32];

        tree.update(&key1, &value1);
        tree.update(&key2, &value2);

        assert_eq!(tree.get(&key1), value1);
        assert_eq!(tree.get(&key2), value2);
    }

    #[test]
    fn test_sparse_merkle_proof() {
        let mut tree = SparseMerkleTree::new(8);

        let key = [1u8; 32];
        let value = [10u8; 32];

        tree.update(&key, &value);

        let proof = tree.generate_proof(&key);
        assert!(tree.verify_proof(&key, &value, &proof));
    }
}
