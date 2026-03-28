//! CRYSTALS-Dilithium signature scheme (NIST PQC standard)
//!
//! Dilithium is a lattice-based digital signature scheme selected by NIST
//! for post-quantum cryptographic standardization. This module uses Dilithium3,
//! which provides NIST security level 3 (equivalent to AES-192).

use pqcrypto_dilithium::dilithium3;
use pqcrypto_traits::sign::{PublicKey as _, SecretKey as _, DetachedSignature};
use serde::{Deserialize, Serialize};
use crate::{CryptoError, Result};

/// Size of a Dilithium3 public key in bytes
pub const PUBLIC_KEY_SIZE: usize = 1952;

/// Size of a Dilithium3 secret key in bytes
pub const SECRET_KEY_SIZE: usize = 4016;

/// Size of a Dilithium3 signature in bytes
pub const SIGNATURE_SIZE: usize = 3293;

/// Dilithium public key for signature verification
#[derive(Clone, PartialEq, Eq)]
pub struct PublicKey(dilithium3::PublicKey);

impl PublicKey {
    /// Create from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        dilithium3::PublicKey::from_bytes(bytes)
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

    /// Derive an address from this public key (first 20 bytes of SHA3-256 hash)
    pub fn to_address(&self) -> [u8; 20] {
        let hash = crate::sha3_256(self.as_bytes());
        let mut addr = [0u8; 20];
        addr.copy_from_slice(&hash[12..32]);
        addr
    }
}

impl std::fmt::Debug for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "PublicKey({} bytes)", self.0.as_bytes().len())
    }
}

impl std::fmt::Display for PublicKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let bytes = self.as_bytes();
        let prefix = &bytes[..4.min(bytes.len())];
        write!(f, "PublicKey({}...)", hex::encode(prefix))
    }
}

impl Serialize for PublicKey {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(self.as_bytes())
    }
}

impl<'de> Deserialize<'de> for PublicKey {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        PublicKey::from_bytes(&bytes).map_err(serde::de::Error::custom)
    }
}

/// Dilithium secret key for signing
pub struct SecretKey(dilithium3::SecretKey);

impl SecretKey {
    /// Create from raw bytes
    pub fn from_bytes(bytes: &[u8]) -> Result<Self> {
        dilithium3::SecretKey::from_bytes(bytes)
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

    /// Get the size of the secret key in bytes
    pub fn len(&self) -> usize {
        self.0.as_bytes().len()
    }

    /// Check if the secret key is empty (always false for valid keys)
    pub fn is_empty(&self) -> bool {
        false
    }
}

impl std::fmt::Debug for SecretKey {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "SecretKey([REDACTED])")
    }
}

impl Clone for SecretKey {
    fn clone(&self) -> Self {
        // Explicitly clone - this should be done carefully
        SecretKey::from_bytes(self.as_bytes()).expect("cloning valid secret key")
    }
}

impl Drop for SecretKey {
    fn drop(&mut self) {
        // Note: pqcrypto handles zeroization internally
        // The underlying library securely zeroes memory on drop
    }
}

/// Dilithium detached signature
#[derive(Clone, PartialEq, Eq)]
pub struct Signature(Vec<u8>);

impl Signature {
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

    /// Get the size of the signature in bytes
    pub fn len(&self) -> usize {
        self.0.len()
    }

    /// Check if the signature is empty
    pub fn is_empty(&self) -> bool {
        self.0.is_empty()
    }

    /// Check if this looks like a valid Dilithium3 signature (by length)
    pub fn is_valid_length(&self) -> bool {
        self.0.len() == SIGNATURE_SIZE
    }
}

impl std::fmt::Debug for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "Signature({} bytes)", self.0.len())
    }
}

impl std::fmt::Display for Signature {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        if self.0.len() >= 4 {
            write!(f, "Signature({}...)", hex::encode(&self.0[..4]))
        } else {
            write!(f, "Signature({})", hex::encode(&self.0))
        }
    }
}

impl Serialize for Signature {
    fn serialize<S>(&self, serializer: S) -> std::result::Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_bytes(&self.0)
    }
}

impl<'de> Deserialize<'de> for Signature {
    fn deserialize<D>(deserializer: D) -> std::result::Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let bytes = Vec::<u8>::deserialize(deserializer)?;
        Ok(Signature::from_bytes(&bytes))
    }
}

/// Dilithium key pair for signing and verification
#[derive(Debug)]
pub struct Keypair {
    /// The public key (safe to share)
    pub public: PublicKey,
    /// The secret key (keep private!)
    pub secret: SecretKey,
}

impl Keypair {
    /// Generate a new random key pair
    /// 
    /// Uses the system's cryptographically secure random number generator.
    pub fn generate() -> Self {
        let (pk, sk) = dilithium3::keypair();
        Self {
            public: PublicKey(pk),
            secret: SecretKey(sk),
        }
    }

    /// Reconstruct a keypair from raw bytes
    pub fn from_bytes(public_bytes: &[u8], secret_bytes: &[u8]) -> Result<Self> {
        Ok(Self {
            public: PublicKey::from_bytes(public_bytes)?,
            secret: SecretKey::from_bytes(secret_bytes)?,
        })
    }

    /// Sign a message
    pub fn sign(&self, message: &[u8]) -> Signature {
        sign(message, &self.secret)
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &Signature) -> bool {
        verify(message, signature, &self.public).is_ok()
    }

    /// Get the public key
    pub fn public_key(&self) -> &PublicKey {
        &self.public
    }

    /// Get the secret key
    pub fn secret_key(&self) -> &SecretKey {
        &self.secret
    }

    /// Derive an address from the public key
    pub fn to_address(&self) -> [u8; 20] {
        self.public.to_address()
    }
}

impl Clone for Keypair {
    fn clone(&self) -> Self {
        Self {
            public: self.public.clone(),
            secret: self.secret.clone(),
        }
    }
}

/// Sign a message with a secret key
/// 
/// Returns a detached signature that can be verified without the original message
/// being embedded in the signature.
pub fn sign(message: &[u8], secret_key: &SecretKey) -> Signature {
    let sig = dilithium3::detached_sign(message, &secret_key.0);
    Signature(sig.as_bytes().to_vec())
}

/// Verify a signature against a message and public key
/// 
/// Returns `Ok(())` if the signature is valid, or an error if invalid.
pub fn verify(message: &[u8], signature: &Signature, public_key: &PublicKey) -> Result<()> {
    let sig = dilithium3::DetachedSignature::from_bytes(&signature.0)
        .map_err(|_| CryptoError::InvalidSignature)?;
    
    dilithium3::verify_detached_signature(&sig, message, &public_key.0)
        .map_err(|_| CryptoError::InvalidSignature)
}

/// Sign a message and return the combined signed message (message + signature)
pub fn sign_attached(message: &[u8], secret_key: &SecretKey) -> Vec<u8> {
    let signed = dilithium3::sign(message, &secret_key.0);
    signed.as_bytes().to_vec()
}

/// Verify and extract the message from a signed message
/// 
/// Returns the original message if verification succeeds.
pub fn verify_attached(signed_message: &[u8], public_key: &PublicKey) -> Result<Vec<u8>> {
    let signed = dilithium3::SignedMessage::from_bytes(signed_message)
        .map_err(|_| CryptoError::InvalidSignature)?;
    
    dilithium3::open(&signed, &public_key.0)
        .map_err(|_| CryptoError::InvalidSignature)
}

// Helper module for hex encoding in Display implementations
mod hex {
    pub fn encode(bytes: &[u8]) -> String {
        bytes.iter().map(|b| format!("{:02x}", b)).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keypair_generation() {
        let keypair = Keypair::generate();
        assert_eq!(keypair.public.len(), PUBLIC_KEY_SIZE);
        assert_eq!(keypair.secret.len(), SECRET_KEY_SIZE);
    }

    #[test]
    fn test_sign_verify() {
        let keypair = Keypair::generate();
        let message = b"Hello, quantum-resistant world!";
        
        let signature = keypair.sign(message);
        assert!(keypair.verify(message, &signature));
        assert!(signature.is_valid_length());
    }

    #[test]
    fn test_invalid_signature() {
        let keypair = Keypair::generate();
        let message = b"Original message";
        let wrong_message = b"Wrong message";
        
        let signature = keypair.sign(message);
        assert!(!keypair.verify(wrong_message, &signature));
    }

    #[test]
    fn test_different_keypairs() {
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::generate();
        let message = b"Test message";
        
        let signature = keypair1.sign(message);
        assert!(!keypair2.verify(message, &signature));
    }

    #[test]
    fn test_sign_attached_verify() {
        let keypair = Keypair::generate();
        let message = b"Test message for attached signature";
        
        let signed = sign_attached(message, &keypair.secret);
        let recovered = verify_attached(&signed, &keypair.public).unwrap();
        
        assert_eq!(recovered, message);
    }

    #[test]
    fn test_keypair_from_bytes() {
        let keypair1 = Keypair::generate();
        let keypair2 = Keypair::from_bytes(
            keypair1.public.as_bytes(),
            keypair1.secret.as_bytes(),
        ).unwrap();
        
        let message = b"Roundtrip test";
        let sig = keypair1.sign(message);
        
        assert!(keypair2.verify(message, &sig));
    }

    #[test]
    fn test_public_key_to_address() {
        let keypair = Keypair::generate();
        let addr = keypair.to_address();
        
        assert_eq!(addr.len(), 20);
        
        // Same keypair produces same address
        let addr2 = keypair.public.to_address();
        assert_eq!(addr, addr2);
    }

    #[test]
    fn test_invalid_public_key_bytes() {
        let result = PublicKey::from_bytes(&[0u8; 100]);
        assert!(result.is_err());
    }

    #[test]
    fn test_invalid_secret_key_bytes() {
        let result = SecretKey::from_bytes(&[0u8; 100]);
        assert!(result.is_err());
    }

    #[test]
    fn test_tampered_signature() {
        let keypair = Keypair::generate();
        let message = b"Original message";
        
        let mut signature = keypair.sign(message);
        
        // Tamper with the signature
        if !signature.0.is_empty() {
            signature.0[0] ^= 0xff;
        }
        
        assert!(!keypair.verify(message, &signature));
    }

    #[test]
    fn test_empty_message() {
        let keypair = Keypair::generate();
        let message = b"";
        
        let signature = keypair.sign(message);
        assert!(keypair.verify(message, &signature));
    }

    #[test]
    fn test_large_message() {
        let keypair = Keypair::generate();
        let message = vec![0xABu8; 1_000_000]; // 1 MB message
        
        let signature = keypair.sign(&message);
        assert!(keypair.verify(&message, &signature));
    }

    #[test]
    fn test_keypair_clone() {
        let keypair1 = Keypair::generate();
        let keypair2 = keypair1.clone();
        
        let message = b"Clone test";
        let sig = keypair1.sign(message);
        
        assert!(keypair2.verify(message, &sig));
    }

    #[test]
    fn test_signature_equality() {
        let keypair = Keypair::generate();
        let message = b"Test message";
        
        let sig1 = keypair.sign(message);
        let sig2 = Signature::from_bytes(sig1.as_bytes());
        
        assert_eq!(sig1, sig2);
    }

    #[test]
    fn test_public_key_equality() {
        let keypair = Keypair::generate();
        let pk1 = &keypair.public;
        let pk2 = PublicKey::from_bytes(pk1.as_bytes()).unwrap();
        
        assert_eq!(*pk1, pk2);
    }
}
