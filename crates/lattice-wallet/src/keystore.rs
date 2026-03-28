//! Encrypted keystore for secure key storage

use aes_gcm::{
    aead::{Aead, KeyInit},
    Aes256Gcm, Nonce,
};
use argon2::{Argon2, Params};
use rand::RngCore;
use serde::{Deserialize, Serialize};
use std::path::Path;
use zeroize::Zeroizing;

use crate::{Result, WalletAccount, WalletError};

/// Argon2 parameters for key derivation
const ARGON2_M_COST: u32 = 65536; // 64 MB
const ARGON2_T_COST: u32 = 3;     // 3 iterations
const ARGON2_P_COST: u32 = 1;     // 1 parallel lane
const SALT_LEN: usize = 32;
const NONCE_LEN: usize = 12;
const KEY_LEN: usize = 32;

/// Keystore version
const KEYSTORE_VERSION: u32 = 1;

/// Encrypted keystore for storing private keys
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Keystore {
    /// Keystore format version
    pub version: u32,
    /// Unique identifier (UUID v4)
    pub id: String,
    /// Address derived from public key (for identification)
    pub address: String,
    /// Crypto parameters
    pub crypto: CryptoParams,
}

/// Cryptographic parameters for encryption
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CryptoParams {
    /// Cipher algorithm (aes-256-gcm)
    pub cipher: String,
    /// Encrypted data (ciphertext)
    #[serde(with = "hex_serde")]
    pub ciphertext: Vec<u8>,
    /// Cipher parameters
    pub cipher_params: CipherParams,
    /// Key derivation function
    pub kdf: String,
    /// KDF parameters
    pub kdf_params: KdfParams,
    /// Message authentication code
    #[serde(with = "hex_serde")]
    pub mac: Vec<u8>,
}

/// Cipher parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CipherParams {
    /// Nonce/IV for AES-GCM
    #[serde(with = "hex_serde")]
    pub nonce: Vec<u8>,
}

/// Key derivation function parameters
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct KdfParams {
    /// Salt for Argon2
    #[serde(with = "hex_serde")]
    pub salt: Vec<u8>,
    /// Memory cost in KiB
    pub m_cost: u32,
    /// Time cost (iterations)
    pub t_cost: u32,
    /// Parallelism
    pub p_cost: u32,
    /// Output length
    pub dklen: u32,
}

/// Hex serialization module
mod hex_serde {
    use serde::{Deserialize, Deserializer, Serializer};

    pub fn serialize<S>(bytes: &Vec<u8>, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        serializer.serialize_str(&hex::encode(bytes))
    }

    pub fn deserialize<'de, D>(deserializer: D) -> Result<Vec<u8>, D::Error>
    where
        D: Deserializer<'de>,
    {
        let s = String::deserialize(deserializer)?;
        hex::decode(&s).map_err(serde::de::Error::custom)
    }
}

impl Keystore {
    /// Create a new keystore from an account and password
    pub fn encrypt(account: &WalletAccount, password: &str) -> Result<Self> {
        let mut rng = rand::thread_rng();
        
        // Generate random salt and nonce
        let mut salt = vec![0u8; SALT_LEN];
        let mut nonce_bytes = vec![0u8; NONCE_LEN];
        rng.fill_bytes(&mut salt);
        rng.fill_bytes(&mut nonce_bytes);
        
        // Derive encryption key using Argon2
        let derived_key = Self::derive_key(password, &salt)?;
        
        // Get secret key bytes
        let secret_bytes = account.secret_key_bytes();
        let public_bytes = account.public_key_bytes();
        
        // Combine public and secret keys for storage
        let mut plaintext = Vec::new();
        plaintext.extend_from_slice(&(public_bytes.len() as u32).to_le_bytes());
        plaintext.extend_from_slice(&public_bytes);
        plaintext.extend_from_slice(&secret_bytes);
        
        // Encrypt with AES-256-GCM
        let cipher = Aes256Gcm::new_from_slice(&derived_key)
            .map_err(|e| WalletError::Encryption(format!("cipher init: {}", e)))?;
        let nonce = Nonce::from_slice(&nonce_bytes);
        
        let ciphertext = cipher
            .encrypt(nonce, plaintext.as_slice())
            .map_err(|e| WalletError::Encryption(format!("encryption failed: {}", e)))?;
        
        // MAC is included in AES-GCM ciphertext (authentication tag)
        // We store it separately for compatibility with standard keystore format
        let mac = lattice_crypto::sha3_256(&[&derived_key[16..], &ciphertext].concat());
        
        let id = uuid::Uuid::new_v4().to_string();
        
        Ok(Self {
            version: KEYSTORE_VERSION,
            id,
            address: account.address().to_base58(),
            crypto: CryptoParams {
                cipher: "aes-256-gcm".to_string(),
                ciphertext,
                cipher_params: CipherParams { nonce: nonce_bytes },
                kdf: "argon2id".to_string(),
                kdf_params: KdfParams {
                    salt,
                    m_cost: ARGON2_M_COST,
                    t_cost: ARGON2_T_COST,
                    p_cost: ARGON2_P_COST,
                    dklen: KEY_LEN as u32,
                },
                mac: mac.to_vec(),
            },
        })
    }

    /// Decrypt keystore and recover the wallet account
    pub fn decrypt(&self, password: &str) -> Result<WalletAccount> {
        // Verify version
        if self.version != KEYSTORE_VERSION {
            return Err(WalletError::InvalidKeystoreFormat(format!(
                "unsupported version: {}",
                self.version
            )));
        }

        // Verify cipher
        if self.crypto.cipher != "aes-256-gcm" {
            return Err(WalletError::InvalidKeystoreFormat(format!(
                "unsupported cipher: {}",
                self.crypto.cipher
            )));
        }

        // Verify KDF
        if self.crypto.kdf != "argon2id" {
            return Err(WalletError::InvalidKeystoreFormat(format!(
                "unsupported kdf: {}",
                self.crypto.kdf
            )));
        }

        // Derive key
        let derived_key = Self::derive_key_with_params(
            password,
            &self.crypto.kdf_params.salt,
            self.crypto.kdf_params.m_cost,
            self.crypto.kdf_params.t_cost,
            self.crypto.kdf_params.p_cost,
        )?;

        // Verify MAC
        let expected_mac = lattice_crypto::sha3_256(
            &[&derived_key[16..], &self.crypto.ciphertext].concat()
        );
        if expected_mac.as_slice() != self.crypto.mac.as_slice() {
            return Err(WalletError::InvalidPassword);
        }

        // Decrypt
        let cipher = Aes256Gcm::new_from_slice(&derived_key)
            .map_err(|e| WalletError::Decryption(format!("cipher init: {}", e)))?;
        let nonce = Nonce::from_slice(&self.crypto.cipher_params.nonce);

        let plaintext = cipher
            .decrypt(nonce, self.crypto.ciphertext.as_slice())
            .map_err(|_| WalletError::InvalidPassword)?;

        // Parse plaintext: [pub_len: u32][public_key][secret_key]
        if plaintext.len() < 4 {
            return Err(WalletError::InvalidKeystoreFormat(
                "plaintext too short".to_string(),
            ));
        }

        let pub_len = u32::from_le_bytes(plaintext[0..4].try_into().unwrap()) as usize;
        if plaintext.len() < 4 + pub_len {
            return Err(WalletError::InvalidKeystoreFormat(
                "invalid public key length".to_string(),
            ));
        }

        let public_bytes = &plaintext[4..4 + pub_len];
        let secret_bytes = &plaintext[4 + pub_len..];

        // Reconstruct keypair
        let public = lattice_crypto::PublicKey::from_bytes(public_bytes)
            .map_err(|e| WalletError::Crypto(format!("invalid public key: {}", e)))?;
        let secret = lattice_crypto::SecretKey::from_bytes(secret_bytes)
            .map_err(|e| WalletError::Crypto(format!("invalid secret key: {}", e)))?;

        let keypair = lattice_crypto::Keypair { public, secret };
        Ok(WalletAccount::from_keypair(keypair))
    }

    /// Derive encryption key using Argon2id
    fn derive_key(password: &str, salt: &[u8]) -> Result<Zeroizing<[u8; KEY_LEN]>> {
        Self::derive_key_with_params(password, salt, ARGON2_M_COST, ARGON2_T_COST, ARGON2_P_COST)
    }

    /// Derive key with custom parameters
    fn derive_key_with_params(
        password: &str,
        salt: &[u8],
        m_cost: u32,
        t_cost: u32,
        p_cost: u32,
    ) -> Result<Zeroizing<[u8; KEY_LEN]>> {
        let params = Params::new(m_cost, t_cost, p_cost, Some(KEY_LEN))
            .map_err(|e| WalletError::Keystore(format!("invalid argon2 params: {}", e)))?;

        let argon2 = Argon2::new(argon2::Algorithm::Argon2id, argon2::Version::V0x13, params);

        let mut key = Zeroizing::new([0u8; KEY_LEN]);
        argon2
            .hash_password_into(password.as_bytes(), salt, &mut *key)
            .map_err(|e| WalletError::Keystore(format!("key derivation failed: {}", e)))?;

        Ok(key)
    }

    /// Save keystore to JSON file
    pub fn save_to_file(&self, path: impl AsRef<Path>) -> Result<()> {
        let json = serde_json::to_string_pretty(self)?;
        std::fs::write(path, json)?;
        Ok(())
    }

    /// Load keystore from JSON file
    pub fn load_from_file(path: impl AsRef<Path>) -> Result<Self> {
        let json = std::fs::read_to_string(path)?;
        let keystore = serde_json::from_str(&json)?;
        Ok(keystore)
    }

    /// Get the address associated with this keystore
    pub fn address(&self) -> &str {
        &self.address
    }

    /// Get the keystore UUID
    pub fn id(&self) -> &str {
        &self.id
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_keystore_encrypt_decrypt() {
        let account = WalletAccount::generate();
        let original_address = account.address().clone();
        let password = "test_password_123!";

        // Encrypt
        let keystore = Keystore::encrypt(&account, password).unwrap();
        assert_eq!(keystore.version, KEYSTORE_VERSION);
        assert_eq!(keystore.address, original_address.to_base58());

        // Decrypt
        let recovered = keystore.decrypt(password).unwrap();
        assert_eq!(recovered.address(), &original_address);
    }

    #[test]
    fn test_keystore_wrong_password() {
        let account = WalletAccount::generate();
        let password = "correct_password";
        let wrong_password = "wrong_password";

        let keystore = Keystore::encrypt(&account, password).unwrap();
        let result = keystore.decrypt(wrong_password);
        
        assert!(matches!(result, Err(WalletError::InvalidPassword)));
    }

    #[test]
    fn test_keystore_json_roundtrip() {
        let account = WalletAccount::generate();
        let password = "secure_password";

        let keystore = Keystore::encrypt(&account, password).unwrap();
        let json = serde_json::to_string_pretty(&keystore).unwrap();
        let loaded: Keystore = serde_json::from_str(&json).unwrap();

        assert_eq!(keystore.id, loaded.id);
        assert_eq!(keystore.address, loaded.address);
        
        // Verify we can still decrypt
        let recovered = loaded.decrypt(password).unwrap();
        assert_eq!(recovered.address(), account.address());
    }
}
