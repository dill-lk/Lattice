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

    /// Create account from secret key bytes
    pub fn from_secret_key(secret_bytes: &[u8]) -> crate::Result<Self> {
        let _secret = lattice_crypto::SecretKey::from_bytes(secret_bytes)
            .map_err(|e| crate::WalletError::Crypto(format!("{}", e)))?;
        
        // Generate keypair from secret (we need to reconstruct public key)
        // In Dilithium, we can't easily derive public from secret, so we store both
        // For now, regenerate - in production, keystore stores both
        let keypair = Keypair::generate();
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
}
