//! Lattice Crypto - Post-quantum cryptography primitives
//!
//! This crate provides quantum-resistant cryptographic operations for the Lattice blockchain:
//!
//! # Signing (Dilithium)
//! - [`Keypair`] - Dilithium key pair for signing
//! - [`PublicKey`] - Public key for signature verification
//! - [`SecretKey`] - Secret key for signing (keep private!)
//! - [`Signature`] - Detached signature
//! - [`sign`] / [`verify`] - Signature operations
//!
//! # Key Encapsulation (Kyber)
//! - [`KyberKeypair`] - Kyber key pair for key encapsulation
//! - [`KyberPublicKey`] / [`KyberSecretKey`] - Kyber keys
//! - [`encapsulate`] / [`decapsulate`] - Key encapsulation operations
//!
//! # Hashing (SHA3-256)
//! - [`sha3_256`] - Compute SHA3-256 hash
//! - [`double_sha3_256`] - Double hash for checksums
//! - [`hash_concat`] - Hash concatenation for Merkle trees
//! - [`Hash`] - 32-byte hash type
//! - [`Hasher`] - Incremental hasher
//!
//! # Example
//!
//! ```ignore
//! use lattice_crypto::{Keypair, sha3_256};
//!
//! // Generate a signing keypair
//! let keypair = Keypair::generate();
//!
//! // Sign a message
//! let message = b"Hello, quantum-resistant world!";
//! let signature = keypair.sign(message);
//!
//! // Verify the signature
//! assert!(keypair.verify(message, &signature));
//!
//! // Hash some data
//! let hash = sha3_256(b"some data");
//! ```

mod dilithium;
mod hash;
mod kyber;

// Re-export Dilithium signing types and functions
pub use dilithium::{
    Keypair, SecretKey, PublicKey, Signature,
    sign, verify, sign_attached, verify_attached,
    PUBLIC_KEY_SIZE, SECRET_KEY_SIZE, SIGNATURE_SIZE,
};

// Re-export SHA3 hashing types and functions
pub use hash::{
    sha3_256, sha3_256_multi, Hash, Hasher,
    double_sha3_256, hash_concat, hash_all,
    hash_to_hex, hash_from_hex,
    HASH_LENGTH, ZERO_HASH, is_zero_hash,
};

// Re-export Kyber KEM types and functions
pub use kyber::{
    KyberKeypair, KyberPublicKey, KyberSecretKey, KyberCiphertext,
    EncapsulationResult, encapsulate, decapsulate, key_exchange,
    KYBER_PUBLIC_KEY_SIZE, KYBER_SECRET_KEY_SIZE, KYBER_CIPHERTEXT_SIZE, SHARED_SECRET_SIZE,
};

/// Cryptography error type
#[derive(Debug, Clone, PartialEq, Eq, thiserror::Error)]
pub enum CryptoError {
    /// The signature is invalid or corrupted
    #[error("invalid signature")]
    InvalidSignature,
    
    /// The public key is invalid or corrupted
    #[error("invalid public key")]
    InvalidPublicKey,
    
    /// The secret key is invalid or corrupted
    #[error("invalid secret key")]
    InvalidSecretKey,
    
    /// Key generation failed
    #[error("key generation failed")]
    KeyGenerationFailed,
    
    /// Key encapsulation failed
    #[error("encapsulation failed")]
    EncapsulationFailed,
    
    /// Key decapsulation failed (invalid ciphertext or wrong key)
    #[error("decapsulation failed")]
    DecapsulationFailed,
    
    /// Hash parsing failed
    #[error("invalid hash format")]
    InvalidHash,
}

/// Result type for cryptographic operations
pub type Result<T> = std::result::Result<T, CryptoError>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_full_signing_workflow() {
        // Generate keypair
        let keypair = Keypair::generate();
        
        // Sign a message
        let message = b"Integration test message";
        let signature = keypair.sign(message);
        
        // Verify signature
        assert!(keypair.verify(message, &signature));
        
        // Verify using standalone function
        assert!(verify(message, &signature, &keypair.public).is_ok());
        
        // Derive address
        let addr = keypair.to_address();
        assert_eq!(addr.len(), 20);
    }

    #[test]
    fn test_full_kyber_workflow() {
        // Generate keypair
        let keypair = KyberKeypair::generate();
        
        // Encapsulate
        let result = encapsulate(&keypair.public).unwrap();
        
        // Decapsulate
        let shared = decapsulate(&keypair.secret, &result.ciphertext).unwrap();
        
        // Shared secrets should match
        assert_eq!(result.shared_secret, shared);
    }

    #[test]
    fn test_hashing_workflow() {
        let data = b"test data";
        
        // Basic hash
        let hash1 = sha3_256(data);
        assert_eq!(hash1.len(), HASH_LENGTH);
        
        // Double hash
        let hash2 = double_sha3_256(data);
        assert_eq!(hash2, sha3_256(&hash1));
        
        // Hash concat
        let left = sha3_256(b"left");
        let right = sha3_256(b"right");
        let combined = hash_concat(&left, &right);
        assert_ne!(combined, left);
        assert_ne!(combined, right);
        
        // Hex roundtrip
        let hex = hash_to_hex(&hash1);
        let recovered = hash_from_hex(&hex).unwrap();
        assert_eq!(hash1, recovered);
    }

    #[test]
    fn test_error_types() {
        // Test that errors are properly formatted
        let err = CryptoError::InvalidSignature;
        assert_eq!(err.to_string(), "invalid signature");
        
        let err = CryptoError::InvalidPublicKey;
        assert_eq!(err.to_string(), "invalid public key");
        
        let err = CryptoError::DecapsulationFailed;
        assert_eq!(err.to_string(), "decapsulation failed");
    }
}
