//! CRYSTALS-Kyber key encapsulation mechanism (for encrypted P2P communication)
//!
//! Kyber is a lattice-based key encapsulation mechanism (KEM) selected by NIST
//! for post-quantum cryptographic standardization. This module uses Kyber768,
//! which provides NIST security level 3.
//!
//! # Usage
//!
//! ```ignore
//! use lattice_crypto::{KyberKeypair, encapsulate, decapsulate};
//!
//! // Alice generates a keypair
//! let alice = KyberKeypair::generate();
//!
//! // Bob encapsulates a shared secret using Alice's public key
//! let result = encapsulate(&alice.public).unwrap();
//!
//! // Alice decapsulates to get the same shared secret
//! let shared = decapsulate(&alice.secret, &result.ciphertext).unwrap();
//!
//! assert_eq!(result.shared_secret, shared);
//! ```

use pqcrypto_kyber::kyber768;
use pqcrypto_traits::kem::{PublicKey as _, SecretKey as _, SharedSecret, Ciphertext as _};
use serde::{Deserialize, Serialize};
use crate::{CryptoError, Result};

/// Size of a Kyber768 public key in bytes
pub const KYBER_PUBLIC_KEY_SIZE: usize = 1184;

/// Size of a Kyber768 secret key in bytes
pub const KYBER_SECRET_KEY_SIZE: usize = 2400;

/// Size of a Kyber768 ciphertext in bytes
pub const KYBER_CIPHERTEXT_SIZE: usize = 1088;

/// Size of the shared secret in bytes
pub const SHARED_SECRET_SIZE: usize = 32;

/// Kyber public key for key encapsulation
#[derive(Clone, PartialEq)]
pub struct KyberPublicKey(kyber768::PublicKey);

impl Eq for KyberPublicKey {}

impl KyberPublicKey {
    /// Create from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        kyber768::PublicKey::from_bytes(bytes)
            .map(Self)
            .map_err(|_| CryptoError::InvalidPublicKey)
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Get byte vector
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.as_bytes().to_vec()
    }

    /// Get the size of the public key in bytes
    pub fn len(&self) -> usize {
        self.0.as_bytes().len()
    }

    /// Check if the public key is empty (always false for valid keys)
    pub fn is_empty(&self) -> bool {
        false
    }
}

impl std::fmt::Debug for KyberPublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KyberPublicKey({} bytes)", self.0.as_bytes().len())
    }
}

impl Serialize for KyberPublicKey {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.as_bytes())
    }
}

impl<'de> Deserialize<'de> for KyberPublicKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        KyberPublicKey::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

/// Kyber secret key for decapsulation
pub struct KyberSecretKey(kyber768::SecretKey);

impl KyberSecretKey {
    /// Create from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        kyber768::SecretKey::from_bytes(bytes)
            .map(Self)
            .map_err(|_| CryptoError::InvalidSecretKey)
    }

    /// Get raw bytes
    ///
    /// # Security
    /// Be careful when handling raw secret key bytes.
    /// Avoid logging or persisting them without encryption.
    pub fn as_bytes(&self) -> &[u8] {
        self.0.as_bytes()
    }

    /// Get byte vector
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.as_bytes().to_vec()
    }

    /// Get the size of the secret key in bytes
    pub fn len(&self) -> usize {
        self.0.as_bytes().len()
    }

    /// Check if the secret key is empty (always false for valid keys)
    pub fn is_empty(&self) -> bool {
        false
    }
}

impl std::fmt::Debug for KyberSecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KyberSecretKey([REDACTED])")
    }
}

impl Clone for KyberSecretKey {
    fn clone(&self) -> Self {
        // Explicitly clone - this should be done carefully
        KyberSecretKey::from_bytes(self.as_bytes()).expect("cloning valid secret key")
    }
}

impl Drop for KyberSecretKey {
    fn drop(&mut self) {
        // Note: pqcrypto handles zeroization internally
    }
}

/// Kyber key pair for key encapsulation/decapsulation
pub struct KyberKeypair {
    /// The public key (safe to share)
    pub public: KyberPublicKey,
    /// The secret key (keep private!)
    pub secret: KyberSecretKey,
}

impl KyberKeypair {
    /// Generate a new random key pair
    ///
    /// Uses the system's cryptographically secure random number generator.
    pub fn generate() -> Self {
        let (pk, sk) = kyber768::keypair();
        Self {
            public: KyberPublicKey(pk),
            secret: KyberSecretKey(sk),
        }
    }

    /// Reconstruct a keypair from raw bytes
    pub fn from_bytes(public_bytes: &[u8], secret_bytes: &[u8]) -> Result<Self> {
        Ok(Self {
            public: KyberPublicKey::from_bytes(public_bytes)?,
            secret: KyberSecretKey::from_bytes(secret_bytes)?,
        })
    }

    /// Get the public key
    pub fn public_key(&self) -> &KyberPublicKey {
        &self.public
    }

    /// Get the secret key
    pub fn secret_key(&self) -> &KyberSecretKey {
        &self.secret
    }

    /// Encapsulate a shared secret for this keypair's public key
    pub fn encapsulate(&self) -> Result<EncapsulationResult> {
        encapsulate(&self.public)
    }

    /// Decapsulate a shared secret using this keypair's secret key
    pub fn decapsulate(&self, ciphertext: &[u8]) -> Result<[u8; 32]> {
        decapsulate(&self.secret, ciphertext)
    }
}

impl std::fmt::Debug for KyberKeypair {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("KyberKeypair")
            .field("public", &self.public)
            .field("secret", &"[REDACTED]")
            .finish()
    }
}

impl Clone for KyberKeypair {
    fn clone(&self) -> Self {
        Self {
            public: self.public.clone(),
            secret: self.secret.clone(),
        }
    }
}

/// Result of key encapsulation operation
#[derive(Clone)]
pub struct EncapsulationResult {
    /// Shared secret (32 bytes) - use this as the symmetric encryption key
    pub shared_secret: [u8; 32],
    /// Ciphertext to send to the recipient
    pub ciphertext: Vec<u8>,
}

impl EncapsulationResult {
    /// Get the shared secret
    pub fn shared_secret(&self) -> &[u8; 32] {
        &self.shared_secret
    }

    /// Get the ciphertext
    pub fn ciphertext(&self) -> &[u8] {
        &self.ciphertext
    }
}

impl std::fmt::Debug for EncapsulationResult {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EncapsulationResult")
            .field("shared_secret", &"[REDACTED 32 bytes]")
            .field("ciphertext", &format!("{} bytes", self.ciphertext.len()))
            .finish()
    }
}

/// Encapsulate a shared secret for a public key
///
/// This generates a random shared secret and encrypts it for the given public key.
/// Returns both the shared secret (for the sender) and the ciphertext (to send to recipient).
pub fn encapsulate(public_key: &KyberPublicKey) -> Result<EncapsulationResult> {
    let (ss, ct) = kyber768::encapsulate(&public_key.0);
    
    let mut shared_secret = [0u8; 32];
    shared_secret.copy_from_slice(ss.as_bytes());
    
    Ok(EncapsulationResult {
        shared_secret,
        ciphertext: ct.as_bytes().to_vec(),
    })
}

/// Decapsulate a shared secret using a secret key
///
/// Given a ciphertext from `encapsulate`, recovers the shared secret using
/// the corresponding secret key.
pub fn decapsulate(secret_key: &KyberSecretKey, ciphertext: &[u8]) -> Result<[u8; 32]> {
    let ct = kyber768::Ciphertext::from_bytes(ciphertext)
        .map_err(|_| CryptoError::DecapsulationFailed)?;
    
    let ss = kyber768::decapsulate(&ct, &secret_key.0);
    
    let mut shared_secret = [0u8; 32];
    shared_secret.copy_from_slice(ss.as_bytes());
    
    Ok(shared_secret)
}

/// Perform a complete key exchange between two parties
///
/// This is a convenience function that encapsulates using one party's public key
/// and returns both the shared secret and ciphertext.
pub fn key_exchange(recipient_public_key: &KyberPublicKey) -> Result<EncapsulationResult> {
    encapsulate(recipient_public_key)
}

/// Ciphertext wrapper for serialization
#[derive(Clone, PartialEq, Eq)]
pub struct KyberCiphertext(Vec<u8>);

impl KyberCiphertext {
    /// Create from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Self {
        Self(bytes.to_vec())
    }

    /// Get raw bytes
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }

    /// Get byte vector
    pub fn to_vec(&self) -> Vec<u8> {
        self.0.clone()
    }

    /// Check if this looks like a valid Kyber768 ciphertext (by length)
    pub fn is_valid_length(&self) -> bool {
        self.0.len() == KYBER_CIPHERTEXT_SIZE
    }
}

impl std::fmt::Debug for KyberCiphertext {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "KyberCiphertext({} bytes)", self.0.len())
    }
}

impl Serialize for KyberCiphertext {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for KyberCiphertext {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        Ok(KyberCiphertext::from_bytes(&bytes))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = KyberKeypair::generate();
        assert_eq!(keypair.public.len(), KYBER_PUBLIC_KEY_SIZE);
        assert_eq!(keypair.secret.len(), KYBER_SECRET_KEY_SIZE);
    }

    #[test]
    fn test_kyber_key_exchange() {
        // Alice generates keypair
        let alice = KyberKeypair::generate();
        
        // Bob encapsulates using Alice's public key
        let result = encapsulate(&alice.public).unwrap();
        
        // Alice decapsulates using her secret key
        let shared = decapsulate(&alice.secret, &result.ciphertext).unwrap();
        
        // Both should have the same shared secret
        assert_eq!(result.shared_secret, shared);
    }

    #[test]
    fn test_ciphertext_size() {
        let alice = KyberKeypair::generate();
        let result = encapsulate(&alice.public).unwrap();
        
        assert_eq!(result.ciphertext.len(), KYBER_CIPHERTEXT_SIZE);
    }

    #[test]
    fn test_keypair_from_bytes() {
        let keypair1 = KyberKeypair::generate();
        let keypair2 = KyberKeypair::from_bytes(
            keypair1.public.as_bytes(),
            keypair1.secret.as_bytes(),
        ).unwrap();
        
        // Should be able to decapsulate with reconstructed keypair
        let result = encapsulate(&keypair1.public).unwrap();
        let shared = decapsulate(&keypair2.secret, &result.ciphertext).unwrap();
        
        assert_eq!(result.shared_secret, shared);
    }

    #[test]
    fn test_invalid_ciphertext() {
        let alice = KyberKeypair::generate();
        
        // Invalid ciphertext should fail
        let result = decapsulate(&alice.secret, &[0u8; 100]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_public_key_bytes() {
        let result = KyberPublicKey::from_bytes(&[0u8; 100]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_secret_key_bytes() {
        let result = KyberSecretKey::from_bytes(&[0u8; 100]);
        assert!(result.is_err());
    }

    #[test]
    fn test_different_keypairs_produce_different_secrets() {
        let alice = KyberKeypair::generate();
        let bob = KyberKeypair::generate();
        
        let result_alice = encapsulate(&alice.public).unwrap();
        let result_bob = encapsulate(&bob.public).unwrap();
        
        // Shared secrets should be different (with overwhelming probability)
        assert_ne!(result_alice.shared_secret, result_bob.shared_secret);
    }

    #[test]
    fn test_keypair_methods() {
        let alice = KyberKeypair::generate();
        
        // Test using keypair methods instead of free functions
        let result = alice.encapsulate().unwrap();
        let shared = alice.decapsulate(&result.ciphertext).unwrap();
        
        assert_eq!(result.shared_secret, shared);
    }

    #[test]
    fn test_keypair_clone() {
        let keypair1 = KyberKeypair::generate();
        let keypair2 = keypair1.clone();
        
        // Cloned keypair should work identically
        let result = encapsulate(&keypair1.public).unwrap();
        let shared = decapsulate(&keypair2.secret, &result.ciphertext).unwrap();
        
        assert_eq!(result.shared_secret, shared);
    }

    #[test]
    fn test_public_key_equality() {
        let keypair = KyberKeypair::generate();
        let pk1 = &keypair.public;
        let pk2 = KyberPublicKey::from_bytes(pk1.as_bytes()).unwrap();
        
        assert_eq!(*pk1, pk2);
    }

    #[test]
    fn test_key_exchange_convenience() {
        let alice = KyberKeypair::generate();
        
        let result = key_exchange(&alice.public).unwrap();
        let shared = alice.decapsulate(&result.ciphertext).unwrap();
        
        assert_eq!(result.shared_secret, shared);
    }

    #[test]
    fn test_ciphertext_wrapper() {
        let alice = KyberKeypair::generate();
        let result = encapsulate(&alice.public).unwrap();
        
        let ct = KyberCiphertext::from_bytes(&result.ciphertext);
        assert!(ct.is_valid_length());
        assert_eq!(ct.as_bytes(), &result.ciphertext);
    }

    #[test]
    fn test_multiple_encapsulations() {
        let alice = KyberKeypair::generate();
        
        // Multiple encapsulations should produce different ciphertexts and secrets
        let result1 = encapsulate(&alice.public).unwrap();
        let result2 = encapsulate(&alice.public).unwrap();
        
        // With overwhelming probability, these should be different
        assert_ne!(result1.ciphertext, result2.ciphertext);
        assert_ne!(result1.shared_secret, result2.shared_secret);
        
        // But both should decapsulate correctly
        let shared1 = decapsulate(&alice.secret, &result1.ciphertext).unwrap();
        let shared2 = decapsulate(&alice.secret, &result2.ciphertext).unwrap();
        
        assert_eq!(result1.shared_secret, shared1);
        assert_eq!(result2.shared_secret, shared2);
    }
}
