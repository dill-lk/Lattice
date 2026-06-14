//! Wallet account management

use lattice_core::Address;
use lattice_crypto::Keypair;
use zeroize::Zeroizing;

/// A wallet account containing a keypair and address
#[derive(Debug)]
pub struct WalletAccount {
    /// The Dilithium keypair for signing
    keypair: Keypair,
    /// Derived address from public key
    address: Address,
    /// Current nonce for transaction ordering
    nonce: u64,
}

impl WalletAccount {
    /// Create a new random wallet account
    pub fn generate() -> Self {
        let keypair = Keypair::generate();
        let public_key = keypair.public.to_vec();
        let address = Address::from_public_key(&public_key);
        
        Self {
            keypair,
            address,
            nonce: 0,
        }
    }

    /// Create account from existing keypair
    pub fn from_keypair(keypair: Keypair) -> Self {
        let public_key = keypair.public.to_vec();
        let address = Address::from_public_key(&public_key);
        
        Self {
            keypair,
            address,
            nonce: 0,
        }
    }

    /// Create account from key material bytes.
    ///
    /// For Dilithium, the public key cannot be safely reconstructed from the
    /// secret key alone. This function therefore accepts either:
    /// - combined key material in the format `[pub_len: u32][public_key][secret_key]`
    /// - or returns an error if only a raw secret key was supplied.
    pub fn from_secret_key(secret_bytes: &[u8]) -> crate::Result<Self> {
        if let Ok(secret) = lattice_crypto::SecretKey::from_bytes(secret_bytes) {
            let _ = secret;
            return Err(crate::WalletError::Crypto(
                "raw secret-key import is unsafe for Dilithium because the public key cannot be reconstructed from secret bytes alone; use a keystore or combined key material export instead"
                    .to_string(),
            ));
        }

        if secret_bytes.len() < 4 {
            return Err(crate::WalletError::Crypto(
                "key material payload too short".to_string(),
            ));
        }

        let pub_len = u32::from_le_bytes(
            secret_bytes[0..4]
                .try_into()
                .map_err(|_| crate::WalletError::Crypto("invalid key material header".to_string()))?,
        ) as usize;

        if secret_bytes.len() < 4 + pub_len {
            return Err(crate::WalletError::Crypto(
                "combined key material is truncated".to_string(),
            ));
        }

        let public_bytes = &secret_bytes[4..4 + pub_len];
        let secret_key_bytes = &secret_bytes[4 + pub_len..];
        let keypair = Keypair::from_bytes(public_bytes, secret_key_bytes)
            .map_err(|e| crate::WalletError::Crypto(format!("{}", e)))?;
        Ok(Self::from_keypair(keypair))
    }

    /// Get the account's address
    pub fn address(&self) -> &Address {
        &self.address
    }

    /// Get reference to the keypair
    pub fn keypair(&self) -> &Keypair {
        &self.keypair
    }

    /// Get the public key bytes
    pub fn public_key_bytes(&self) -> Vec<u8> {
        self.keypair.public.to_vec()
    }

    /// Get the secret key bytes (handle with care!)
    pub fn secret_key_bytes(&self) -> Zeroizing<Vec<u8>> {
        Zeroizing::new(self.keypair.secret.as_bytes().to_vec())
    }

    /// Get current nonce
    pub fn nonce(&self) -> u64 {
        self.nonce
    }

    /// Set the nonce (e.g., after syncing with network)
    pub fn set_nonce(&mut self, nonce: u64) {
        self.nonce = nonce;
    }

    /// Increment nonce and return the previous value
    pub fn next_nonce(&mut self) -> u64 {
        let current = self.nonce;
        self.nonce += 1;
        current
    }

    /// Sign arbitrary data
    pub fn sign(&self, message: &[u8]) -> Vec<u8> {
        self.keypair.sign(message).to_vec()
    }

    /// Verify a signature
    pub fn verify(&self, message: &[u8], signature: &[u8]) -> bool {
        let sig = lattice_crypto::Signature::from_bytes(signature);
        self.keypair.verify(message, &sig)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_generate_account() {
        let account = WalletAccount::generate();
        assert!(!account.address().is_zero());
        assert_eq!(account.nonce(), 0);
    }

    #[test]
    fn test_nonce_management() {
        let mut account = WalletAccount::generate();
        assert_eq!(account.nonce(), 0);
        
        let n1 = account.next_nonce();
        assert_eq!(n1, 0);
        assert_eq!(account.nonce(), 1);
        
        let n2 = account.next_nonce();
        assert_eq!(n2, 1);
        assert_eq!(account.nonce(), 2);
        
        account.set_nonce(100);
        assert_eq!(account.nonce(), 100);
    }

    #[test]
    fn test_sign_verify() {
        let account = WalletAccount::generate();
        let message = b"Hello, Lattice!";

        let signature = account.sign(message);
        assert!(account.verify(message, &signature));

        // Wrong message should fail
        assert!(!account.verify(b"Wrong message", &signature));
    }

    #[test]
    fn test_from_combined_key_material() {
        let account = WalletAccount::generate();
        let public_bytes = account.public_key_bytes();
        let secret_bytes = account.secret_key_bytes();

        let mut payload = Vec::new();
        payload.extend_from_slice(&(public_bytes.len() as u32).to_le_bytes());
        payload.extend_from_slice(&public_bytes);
        payload.extend_from_slice(&secret_bytes);

        let recovered = WalletAccount::from_secret_key(&payload).unwrap();
        assert_eq!(recovered.address(), account.address());
    }

    #[test]
    fn test_from_raw_secret_key_rejected() {
        let account = WalletAccount::generate();
        let secret_bytes = account.secret_key_bytes();
        let result = WalletAccount::from_secret_key(&secret_bytes);
        assert!(result.is_err());
    }
}
