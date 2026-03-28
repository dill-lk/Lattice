//! Address type for Lattice blockchain

use crate::PublicKey;
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

/// Address derived from a Dilithium public key
/// 
/// Format: SHA3-256(public_key)[0..20] (20 bytes)
#[derive(Debug, Clone, PartialEq, Eq, PartialOrd, Ord, Hash, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Address([u8; 20]);

impl Address {
    /// Create address from raw bytes
    pub fn from_bytes(bytes: [u8; 20]) -> Self {
        Self(bytes)
    }

    /// Derive address from public key
    pub fn from_public_key(public_key: &PublicKey) -> Self {
        let hash = Sha3_256::digest(public_key);
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&hash[..20]);
        Self(addr)
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8; 20] {
        &self.0
    }

    /// Encode as base58check string
    pub fn to_base58(&self) -> String {
        // Version byte (0x00 for mainnet) + address bytes
        let mut versioned = vec![0x00];
        versioned.extend_from_slice(&self.0);
        
        // Add checksum (first 4 bytes of double SHA3)
        let hash1 = Sha3_256::digest(&versioned);
        let hash2 = Sha3_256::digest(hash1);
        versioned.extend_from_slice(&hash2[..4]);
        
        bs58::encode(versioned).into_string()
    }

    /// Decode from base58check string
    pub fn from_base58(s: &str) -> Result<Self, AddressError> {
        let bytes = bs58::decode(s)
            .into_vec()
            .map_err(|_| AddressError::InvalidBase58)?;
        
        if bytes.len() != 25 {
            return Err(AddressError::InvalidLength);
        }

        // Verify checksum
        let hash1 = Sha3_256::digest(&bytes[..21]);
        let hash2 = Sha3_256::digest(hash1);
        if bytes[21..] != hash2[..4] {
            return Err(AddressError::InvalidChecksum);
        }

        let mut addr = [0u8; 20];
        addr.copy_from_slice(&bytes[1..21]);
        Ok(Self(addr))
    }

    /// Zero address (used for coinbase transactions)
    pub fn zero() -> Self {
        Self([0u8; 20])
    }

    /// Check if this is the zero address
    pub fn is_zero(&self) -> bool {
        self.0 == [0u8; 20]
    }
}

impl std::fmt::Display for Address {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.to_base58())
    }
}

#[derive(Debug, thiserror::Error)]
pub enum AddressError {
    #[error("invalid base58 encoding")]
    InvalidBase58,
    #[error("invalid address length")]
    InvalidLength,
    #[error("invalid checksum")]
    InvalidChecksum,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_address_roundtrip() {
        let addr = Address::from_bytes([1u8; 20]);
        let encoded = addr.to_base58();
        let decoded = Address::from_base58(&encoded).unwrap();
        assert_eq!(addr, decoded);
    }

    #[test]
    fn test_zero_address() {
        let zero = Address::zero();
        assert!(zero.is_zero());
    }
}
