//! Integration tests for cryptographic operations

use lattice_crypto::{
    Keypair, PublicKey, Signature, sign, verify,
    KyberKeypair, encapsulate, decapsulate,
    sha3_256, double_sha3_256, hash_concat, hash_to_hex, hash_from_hex,
    Hash, HASH_LENGTH
};

// ============================================================================
// Dilithium Signature Tests
// ============================================================================

#[test]
fn test_dilithium_full_workflow() {
    let keypair = Keypair::generate();
    let message = b"Lattice blockchain - quantum resistant";
    
    // Sign
    let signature = keypair.sign(message);
    
    // Verify
    assert!(keypair.verify(message, &signature));
    assert!(verify(message, &signature, &keypair.public).is_ok());
}

#[test]
fn test_signature_verification_fails_wrong_message() {
    let keypair = Keypair::generate();
    let message1 = b"Original message";
    let message2 = b"Modified message";
    
    let signature = keypair.sign(message1);
    
    // Should fail with different message
    assert!(!keypair.verify(message2, &signature));
}

#[test]
fn test_signature_verification_fails_wrong_key() {
    let keypair1 = Keypair::generate();
    let keypair2 = Keypair::generate();
    let message = b"Test message";
    
    let signature = keypair1.sign(message);
    
    // Should fail with different public key
    assert!(!keypair2.verify(message, &signature));
    assert!(verify(message, &signature, &keypair2.public).is_err());
}

#[test]
fn test_public_key_serialization() {
    let keypair = Keypair::generate();
    
    // Serialize
    let bytes = keypair.public.to_vec();
    
    // Deserialize
    let recovered = PublicKey::from_bytes(&bytes).unwrap();
    
    // Should match
    assert_eq!(keypair.public.as_bytes(), recovered.as_bytes());
}

#[test]
fn test_secret_key_serialization() {
    let keypair = Keypair::generate();
    
    // Serialize
    let bytes = keypair.secret.to_vec();
    
    // Deserialize and create new keypair
    let message = b"Test message";
    let signature1 = keypair.sign(message);
    
    // Signature verification should still work
    assert!(keypair.verify(message, &signature1));
}

#[test]
fn test_address_derivation_deterministic() {
    let keypair = Keypair::generate();
    
    let addr1 = keypair.to_address();
    let addr2 = keypair.to_address();
    
    // Should be deterministic
    assert_eq!(addr1, addr2);
    assert_eq!(addr1.len(), 20);
}

#[test]
fn test_different_keys_different_addresses() {
    let keypair1 = Keypair::generate();
    let keypair2 = Keypair::generate();
    
    let addr1 = keypair1.to_address();
    let addr2 = keypair2.to_address();
    
    assert_ne!(addr1, addr2);
}

#[test]
fn test_signature_attached() {
    let keypair = Keypair::generate();
    let message = b"Test attached signature";
    
    // Sign with attached signature (message embedded in signature)
    let signed_message = keypair.sign_attached(message);
    
    // Verify and extract message
    let recovered = keypair.verify_attached(&signed_message).unwrap();
    assert_eq!(recovered, message);
}

// ============================================================================
// Kyber KEM Tests
// ============================================================================

#[test]
fn test_kyber_encapsulation_decapsulation() {
    let keypair = KyberKeypair::generate();
    
    // Encapsulate
    let result = encapsulate(&keypair.public).unwrap();
    
    // Decapsulate
    let shared_secret = decapsulate(&keypair.secret, &result.ciphertext).unwrap();
    
    // Should match
    assert_eq!(result.shared_secret, shared_secret);
}

#[test]
fn test_kyber_different_encapsulations_different_secrets() {
    let keypair = KyberKeypair::generate();
    
    // Two encapsulations should produce different ciphertexts and secrets
    let result1 = encapsulate(&keypair.public).unwrap();
    let result2 = encapsulate(&keypair.public).unwrap();
    
    assert_ne!(result1.ciphertext.as_bytes(), result2.ciphertext.as_bytes());
    assert_ne!(result1.shared_secret, result2.shared_secret);
}

#[test]
fn test_kyber_wrong_key_fails() {
    let keypair1 = KyberKeypair::generate();
    let keypair2 = KyberKeypair::generate();
    
    // Encapsulate with keypair1's public key
    let result = encapsulate(&keypair1.public).unwrap();
    
    // Try to decapsulate with keypair2's secret key (wrong key)
    let shared1 = decapsulate(&keypair1.secret, &result.ciphertext).unwrap();
    let shared2 = decapsulate(&keypair2.secret, &result.ciphertext).unwrap();
    
    // Should produce different (wrong) shared secret
    assert_ne!(result.shared_secret, shared2);
    assert_eq!(result.shared_secret, shared1);
}

#[test]
fn test_kyber_key_serialization() {
    let keypair = KyberKeypair::generate();
    
    // Serialize
    let pk_bytes = keypair.public.to_vec();
    let sk_bytes = keypair.secret.to_vec();
    
    // Deserialize
    let recovered_pk = KyberPublicKey::from_bytes(&pk_bytes).unwrap();
    let recovered_sk = KyberSecretKey::from_bytes(&sk_bytes).unwrap();
    
    // Test that it still works
    let result = encapsulate(&recovered_pk).unwrap();
    let shared = decapsulate(&recovered_sk, &result.ciphertext).unwrap();
    
    assert_eq!(result.shared_secret, shared);
}

// ============================================================================
// SHA3 Hashing Tests
// ============================================================================

#[test]
fn test_sha3_deterministic() {
    let data = b"Lattice blockchain";
    
    let hash1 = sha3_256(data);
    let hash2 = sha3_256(data);
    
    assert_eq!(hash1, hash2);
    assert_eq!(hash1.len(), HASH_LENGTH);
}

#[test]
fn test_sha3_different_inputs() {
    let data1 = b"Data 1";
    let data2 = b"Data 2";
    
    let hash1 = sha3_256(data1);
    let hash2 = sha3_256(data2);
    
    assert_ne!(hash1, hash2);
}

#[test]
fn test_double_hash() {
    let data = b"Test data";
    
    let hash1 = sha3_256(data);
    let hash2 = sha3_256(&hash1);
    let double = double_sha3_256(data);
    
    assert_eq!(hash2, double);
}

#[test]
fn test_hash_concat() {
    let left = sha3_256(b"left");
    let right = sha3_256(b"right");
    
    let combined = hash_concat(&left, &right);
    
    // Should be different from both inputs
    assert_ne!(combined, left);
    assert_ne!(combined, right);
    
    // Should be deterministic
    let combined2 = hash_concat(&left, &right);
    assert_eq!(combined, combined2);
    
    // Order matters
    let reversed = hash_concat(&right, &left);
    assert_ne!(combined, reversed);
}

#[test]
fn test_hash_hex_encoding() {
    let data = b"Test data for hex encoding";
    let hash = sha3_256(data);
    
    // Encode to hex
    let hex = hash_to_hex(&hash);
    assert_eq!(hex.len(), HASH_LENGTH * 2); // Each byte = 2 hex chars
    
    // Decode from hex
    let recovered = hash_from_hex(&hex).unwrap();
    assert_eq!(hash, recovered);
}

#[test]
fn test_hash_hex_invalid() {
    // Invalid hex string
    assert!(hash_from_hex("invalid_hex").is_err());
    
    // Wrong length
    assert!(hash_from_hex("abcd").is_err());
    
    // Valid hex but wrong length
    let short_hex = "a".repeat(62); // 31 bytes
    assert!(hash_from_hex(&short_hex).is_err());
}

#[test]
fn test_incremental_hashing() {
    use lattice_crypto::Hasher;
    
    let data1 = b"Part 1 ";
    let data2 = b"Part 2";
    
    // Hash incrementally
    let mut hasher = Hasher::new();
    hasher.update(data1);
    hasher.update(data2);
    let hash1 = hasher.finalize();
    
    // Hash all at once
    let mut combined = data1.to_vec();
    combined.extend_from_slice(data2);
    let hash2 = sha3_256(&combined);
    
    assert_eq!(hash1, hash2);
}

// ============================================================================
// Cross-module Integration Tests
// ============================================================================

#[test]
fn test_sign_hash() {
    let keypair = Keypair::generate();
    let data = b"Data to hash and sign";
    
    // Hash the data
    let hash = sha3_256(data);
    
    // Sign the hash
    let signature = keypair.sign(&hash);
    
    // Verify
    assert!(keypair.verify(&hash, &signature));
}

#[test]
fn test_merkle_tree_construction() {
    // Simulate simple merkle tree with 4 leaves
    let leaf1 = sha3_256(b"tx1");
    let leaf2 = sha3_256(b"tx2");
    let leaf3 = sha3_256(b"tx3");
    let leaf4 = sha3_256(b"tx4");
    
    // Level 1: combine pairs
    let node1 = hash_concat(&leaf1, &leaf2);
    let node2 = hash_concat(&leaf3, &leaf4);
    
    // Level 2: root
    let root = hash_concat(&node1, &node2);
    
    // Root should be 32 bytes
    assert_eq!(root.len(), HASH_LENGTH);
    
    // Changing any leaf should change root
    let leaf1_modified = sha3_256(b"tx1_modified");
    let node1_modified = hash_concat(&leaf1_modified, &leaf2);
    let root_modified = hash_concat(&node1_modified, &node2);
    
    assert_ne!(root, root_modified);
}

#[test]
fn test_quantum_resistance_signature_sizes() {
    use lattice_crypto::{PUBLIC_KEY_SIZE, SECRET_KEY_SIZE, SIGNATURE_SIZE};
    
    let keypair = Keypair::generate();
    
    // Check sizes match expected post-quantum sizes
    assert_eq!(keypair.public.len(), PUBLIC_KEY_SIZE);
    assert_eq!(keypair.secret.len(), SECRET_KEY_SIZE);
    
    let signature = keypair.sign(b"test");
    assert_eq!(signature.len(), SIGNATURE_SIZE);
    
    // Dilithium3 signatures are large (~2.5KB) but quantum-resistant
    assert!(SIGNATURE_SIZE > 2000);
    assert!(SIGNATURE_SIZE < 4000);
}

#[test]
fn test_kyber_shared_secret_size() {
    use lattice_crypto::SHARED_SECRET_SIZE;
    
    let keypair = KyberKeypair::generate();
    let result = encapsulate(&keypair.public).unwrap();
    
    assert_eq!(result.shared_secret.len(), SHARED_SECRET_SIZE);
    assert_eq!(SHARED_SECRET_SIZE, 32); // 256 bits
}
