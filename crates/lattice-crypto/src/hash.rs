//! SHA3-256 hashing

use sha3::{Digest, Sha3_256};

/// SHA3-256 hash output (32 bytes)
pub type Hash = [u8; 32];

/// Length of a SHA3-256 hash in bytes
pub const HASH_LENGTH: usize = 32;

/// SHA3-256 hasher wrapper for incremental hashing
#[derive(Clone)]
pub struct Hasher(Sha3_256);

impl Hasher {
    /// Create a new hasher
    pub fn new() -> Self {
        Self(Sha3_256::new())
    }

    /// Update with data
    pub fn update(&mut self, data: &[u8]) {
        self.0.update(data);
    }

    /// Chain update for builder pattern
    pub fn chain(mut self, data: &[u8]) -> Self {
        self.0.update(data);
        self
    }

    /// Finalize and return hash
    pub fn finalize(self) -> Hash {
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&self.0.finalize());
        hash
    }

    /// Reset the hasher to initial state
    pub fn reset(&mut self) {
        self.0 = Sha3_256::new();
    }
}

impl Default for Hasher {
    fn default() -> Self {
        Self::new()
    }
}

/// Compute SHA3-256 hash of data
pub fn sha3_256(data: &[u8]) -> Hash {
    let mut hash = [0u8; 32];
    hash.copy_from_slice(&Sha3_256::digest(data));
    hash
}

/// Compute SHA3-256 hash of multiple chunks
pub fn sha3_256_multi(chunks: &[&[u8]]) -> Hash {
    let mut hasher = Hasher::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize()
}

/// Compute double SHA3-256 hash (hash of hash) for checksums
/// 
/// This is commonly used in blockchain for additional security:
/// `sha3_256(sha3_256(data))`
pub fn double_sha3_256(data: &[u8]) -> Hash {
    sha3_256(&sha3_256(data))
}

/// Concatenate two hashes and hash the result
/// 
/// Useful for building Merkle trees: `hash(left || right)`
pub fn hash_concat(left: &Hash, right: &Hash) -> Hash {
    let mut hasher = Hasher::new();
    hasher.update(left);
    hasher.update(right);
    hasher.finalize()
}

/// Hash a slice of hashes (for Merkle tree leaves)
pub fn hash_all(hashes: &[Hash]) -> Hash {
    let mut hasher = Hasher::new();
    for h in hashes {
        hasher.update(h);
    }
    hasher.finalize()
}

/// Convert a hash to a hexadecimal string
pub fn hash_to_hex(hash: &Hash) -> String {
    hash.iter().map(|b| format!("{:02x}", b)).collect()
}

/// Parse a hash from a hexadecimal string
pub fn hash_from_hex(hex: &str) -> Option<Hash> {
    if hex.len() != 64 {
        return None;
    }
    
    let mut hash = [0u8; 32];
    for (i, chunk) in hex.as_bytes().chunks(2).enumerate() {
        let s = std::str::from_utf8(chunk).ok()?;
        hash[i] = u8::from_str_radix(s, 16).ok()?;
    }
    Some(hash)
}

/// Zero hash constant (all zeros)
pub const ZERO_HASH: Hash = [0u8; 32];

/// Check if a hash is all zeros
pub fn is_zero_hash(hash: &Hash) -> bool {
    hash == &ZERO_HASH
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_sha3_256() {
        let hash = sha3_256(b"hello");
        assert_eq!(hash.len(), 32);
        
        // SHA3-256("hello") is deterministic
        let hash2 = sha3_256(b"hello");
        assert_eq!(hash, hash2);
    }

    #[test]
    fn test_hasher_incremental() {
        let hash1 = sha3_256(b"helloworld");
        
        let mut hasher = Hasher::new();
        hasher.update(b"hello");
        hasher.update(b"world");
        let hash2 = hasher.finalize();
        
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_hasher_chain() {
        let hash1 = sha3_256(b"helloworld");
        let hash2 = Hasher::new()
            .chain(b"hello")
            .chain(b"world")
            .finalize();
        
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_double_sha3_256() {
        let data = b"test data";
        let single = sha3_256(data);
        let double = double_sha3_256(data);
        
        // Double hash should equal hash of single hash
        assert_eq!(double, sha3_256(&single));
        // But should differ from single hash
        assert_ne!(single, double);
    }

    #[test]
    fn test_hash_concat() {
        let left = sha3_256(b"left");
        let right = sha3_256(b"right");
        
        let combined = hash_concat(&left, &right);
        
        // Should be deterministic
        let combined2 = hash_concat(&left, &right);
        assert_eq!(combined, combined2);
        
        // Order matters
        let reversed = hash_concat(&right, &left);
        assert_ne!(combined, reversed);
    }

    #[test]
    fn test_hash_to_hex_roundtrip() {
        let original = sha3_256(b"test");
        let hex = hash_to_hex(&original);
        let recovered = hash_from_hex(&hex).unwrap();
        
        assert_eq!(original, recovered);
        assert_eq!(hex.len(), 64);
    }

    #[test]
    fn test_hash_from_hex_invalid() {
        assert!(hash_from_hex("").is_none());
        assert!(hash_from_hex("abc").is_none());
        assert!(hash_from_hex("gg00000000000000000000000000000000000000000000000000000000000000").is_none());
    }

    #[test]
    fn test_zero_hash() {
        assert!(is_zero_hash(&ZERO_HASH));
        assert!(!is_zero_hash(&sha3_256(b"not zero")));
    }

    #[test]
    fn test_hash_all() {
        let h1 = sha3_256(b"one");
        let h2 = sha3_256(b"two");
        let h3 = sha3_256(b"three");
        
        let result = hash_all(&[h1, h2, h3]);
        
        // Should be equivalent to manual concatenation
        let mut hasher = Hasher::new();
        hasher.update(&h1);
        hasher.update(&h2);
        hasher.update(&h3);
        let expected = hasher.finalize();
        
        assert_eq!(result, expected);
    }

    #[test]
    fn test_hasher_reset() {
        let mut hasher = Hasher::new();
        hasher.update(b"some data");
        hasher.reset();
        hasher.update(b"hello");
        let hash1 = hasher.finalize();
        
        let hash2 = sha3_256(b"hello");
        assert_eq!(hash1, hash2);
    }
}
