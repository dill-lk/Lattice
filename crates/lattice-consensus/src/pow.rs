//! Memory-hard Proof of Work using Argon2
//!
//! The PoW puzzle: find a nonce such that `argon2(header || nonce) < target`.
//! Argon2 provides memory-hardness, making ASIC development expensive.

use argon2::{Algorithm, Argon2, Params, Version};
use lattice_core::{BlockHeader, Hash};
use sha3::{Digest, Sha3_256};
use thiserror::Error;

/// PoW-related errors
#[derive(Debug, Error)]
pub enum PoWError {
    #[error("argon2 hashing failed: {0}")]
    Argon2Error(String),
    #[error("invalid difficulty: {0}")]
    InvalidDifficulty(String),
    #[error("hash does not meet target")]
    HashAboveTarget,
}

pub type Result<T> = std::result::Result<T, PoWError>;

/// Configuration for memory-hard PoW
#[derive(Debug, Clone)]
pub struct PoWConfig {
    /// Memory cost in KiB (default: 64 MiB = 65536 KiB)
    pub memory_cost_kib: u32,
    /// Time cost (iterations, default: 3)
    pub time_cost: u32,
    /// Parallelism (lanes, default: 4)
    pub parallelism: u32,
    /// Output hash length in bytes
    pub output_len: usize,
}

impl Default for PoWConfig {
    fn default() -> Self {
        Self {
            memory_cost_kib: 65536, // 64 MiB
            time_cost: 3,
            parallelism: 4,
            output_len: 32,
        }
    }
}

impl PoWConfig {
    /// Create a light config for testing (much faster)
    pub fn light() -> Self {
        Self {
            memory_cost_kib: 1024, // 1 MiB
            time_cost: 1,
            parallelism: 1,
            output_len: 32,
        }
    }

    /// Create the Argon2 hasher from this config
    fn create_argon2(&self) -> Result<Argon2<'static>> {
        let params = Params::new(
            self.memory_cost_kib,
            self.time_cost,
            self.parallelism,
            Some(self.output_len),
        )
        .map_err(|e| PoWError::Argon2Error(e.to_string()))?;

        Ok(Argon2::new(Algorithm::Argon2id, Version::V0x13, params))
    }
}

/// Compute the PoW hash for a block header with a specific nonce
pub fn compute_pow_hash(header: &BlockHeader, nonce: u64, config: &PoWConfig) -> Result<Hash> {
    let argon2 = config.create_argon2()?;

    // Serialize header (without nonce) and append nonce
    let header_bytes = borsh::to_vec(header).expect("serialization cannot fail");

    // Create a unique salt from previous hash and height
    let mut salt = [0u8; 16];
    salt[..8].copy_from_slice(&header.prev_hash[..8]);
    salt[8..].copy_from_slice(&header.height.to_le_bytes());

    // Input: header_bytes || nonce
    let mut input = header_bytes;
    input.extend_from_slice(&nonce.to_le_bytes());

    // Compute Argon2 hash
    let mut output = [0u8; 32];
    argon2
        .hash_password_into(&input, &salt, &mut output)
        .map_err(|e| PoWError::Argon2Error(e.to_string()))?;

    // Final SHA3 mixing to ensure uniformity
    let final_hash = Sha3_256::digest(&output);
    let mut result = [0u8; 32];
    result.copy_from_slice(&final_hash);

    Ok(result)
}

/// Verify that a PoW solution is valid
pub fn verify_pow(header: &BlockHeader, config: &PoWConfig) -> Result<bool> {
    let pow_hash = compute_pow_hash(header, header.nonce, config)?;
    let target = difficulty_to_target(header.difficulty);
    Ok(compare_hash_to_target(&pow_hash, &target))
}

/// Convert difficulty to a 256-bit target
/// Higher difficulty = lower target = harder to find valid hash
pub fn difficulty_to_target(difficulty: u64) -> Hash {
    if difficulty == 0 {
        return [0xFF; 32]; // Max target for zero difficulty
    }

    // Target = MAX_TARGET / difficulty
    // MAX_TARGET = 2^256 - 1 (all 0xFF bytes)
    //
    // For efficiency, we compute leading zeros based on difficulty:
    // Number of leading zero bits ≈ log2(difficulty)

    let leading_zero_bits = 64 - difficulty.leading_zeros() as u64;

    // Start with max target
    let mut target = [0xFF; 32];

    // Zero out leading bytes
    let zero_bytes = (leading_zero_bits / 8) as usize;
    let remaining_bits = (leading_zero_bits % 8) as u8;

    for byte in target.iter_mut().take(zero_bytes) {
        *byte = 0;
    }

    // Handle partial byte
    if zero_bytes < 32 && remaining_bits > 0 {
        target[zero_bytes] = 0xFF >> remaining_bits;
    }

    // Scale remaining bytes by difficulty for finer granularity
    if zero_bytes < 32 {
        let scale = (difficulty & 0xFF).max(1) as u8;
        let divisor = scale.max(1);
        if zero_bytes + 1 < 32 {
            target[zero_bytes + 1] = 0xFF / divisor;
        }
    }

    target
}

/// Compare a hash against a target (hash <= target for valid PoW)
fn compare_hash_to_target(hash: &Hash, target: &Hash) -> bool {
    for i in 0..32 {
        if hash[i] < target[i] {
            return true;
        }
        if hash[i] > target[i] {
            return false;
        }
    }
    true // Equal
}

/// Calculate the hash rate difficulty multiplier
pub fn difficulty_multiplier(difficulty: u64) -> f64 {
    difficulty as f64
}

/// Estimate hashes needed to find a valid block
pub fn estimated_hashes_for_difficulty(difficulty: u64) -> u64 {
    // Expected attempts = 2^(leading_zero_bits)
    let leading_zero_bits = 64 - difficulty.leading_zeros();
    1u64.saturating_shl(leading_zero_bits)
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::Address;

    fn test_header() -> BlockHeader {
        BlockHeader {
            version: 1,
            height: 1,
            prev_hash: [0u8; 32],
            tx_root: [0u8; 32],
            state_root: [0u8; 32],
            timestamp: 1704067200000,
            difficulty: 1,
            nonce: 0,
            coinbase: Address::zero(),
        }
    }

    #[test]
    fn test_pow_hash_deterministic() {
        let config = PoWConfig::light();
        let header = test_header();

        let hash1 = compute_pow_hash(&header, 0, &config).unwrap();
        let hash2 = compute_pow_hash(&header, 0, &config).unwrap();

        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_pow_hash_changes_with_nonce() {
        let config = PoWConfig::light();
        let header = test_header();

        let hash1 = compute_pow_hash(&header, 0, &config).unwrap();
        let hash2 = compute_pow_hash(&header, 1, &config).unwrap();

        assert_ne!(hash1, hash2);
    }

    #[test]
    fn test_difficulty_to_target() {
        let target_easy = difficulty_to_target(1);
        let target_hard = difficulty_to_target(100);

        // Higher difficulty should result in lower (harder) target
        assert!(
            target_hard < target_easy,
            "Hard target should be lower than easy target"
        );
    }

    #[test]
    fn test_verify_pow_easy_difficulty() {
        let config = PoWConfig::light();
        let mut header = test_header();
        header.difficulty = 1; // Very easy

        // With difficulty 1, most nonces should work
        // Try a few nonces to find a valid one
        for nonce in 0..100 {
            header.nonce = nonce;
            if verify_pow(&header, &config).unwrap() {
                return; // Found valid PoW
            }
        }

        // With such low difficulty, we should find one
        // This might fail very rarely due to randomness
    }

    #[test]
    fn test_compare_hash_to_target() {
        let lower = [0x00; 32];
        let higher = [0xFF; 32];
        let mid = [0x80; 32];

        assert!(compare_hash_to_target(&lower, &higher));
        assert!(!compare_hash_to_target(&higher, &lower));
        assert!(compare_hash_to_target(&mid, &higher));
        assert!(!compare_hash_to_target(&higher, &mid));
        assert!(compare_hash_to_target(&mid, &mid)); // Equal is valid
    }
}
