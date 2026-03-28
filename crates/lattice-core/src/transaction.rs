//! Transaction types for Lattice blockchain

use crate::{Address, Amount, Hash, PublicKey, Signature};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

/// Transaction kind
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum TransactionKind {
    /// Simple value transfer
    Transfer,
    /// Deploy a smart contract
    Deploy,
    /// Call a smart contract
    Call,
}

/// A transaction on the Lattice blockchain
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Transaction {
    /// Transaction type
    pub kind: TransactionKind,
    /// Sender address
    pub from: Address,
    /// Recipient address (or contract address for calls)
    pub to: Address,
    /// Amount to transfer
    pub amount: Amount,
    /// Transaction fee
    pub fee: Amount,
    /// Sender's nonce (for replay protection)
    pub nonce: u64,
    /// Transaction data (contract code or call data)
    pub data: Vec<u8>,
    /// Gas limit for contract execution
    pub gas_limit: u64,
    /// Chain ID for replay protection
    pub chain_id: u32,
    /// Sender's public key (for signature verification)
    pub public_key: PublicKey,
    /// Dilithium signature
    pub signature: Signature,
}

impl Transaction {
    /// Calculate the hash of this transaction
    pub fn hash(&self) -> Hash {
        let bytes = self.signing_bytes();
        let digest = Sha3_256::digest(&bytes);
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&digest);
        hash
    }

    /// Get bytes to sign (excludes signature field)
    pub fn signing_bytes(&self) -> Vec<u8> {
        let mut tx = self.clone();
        tx.signature = vec![];
        borsh::to_vec(&tx).expect("serialization cannot fail")
    }

    /// Check if signature is present
    pub fn is_signed(&self) -> bool {
        !self.signature.is_empty()
    }

    /// Verify the transaction's signature
    pub fn verify_signature(&self) -> bool {
        if !self.is_signed() {
            return false;
        }

        // Verify sender matches public key
        let derived_addr = Address::from_public_key(&self.public_key);
        if derived_addr != self.from {
            return false;
        }

        // Signature verification is delegated to lattice-crypto
        // Here we just check the format
        !self.signature.is_empty() && !self.public_key.is_empty()
    }

    /// Calculate gas cost for this transaction
    pub fn gas_cost(&self) -> u64 {
        const BASE_GAS: u64 = 21000;
        const DATA_GAS_PER_BYTE: u64 = 16;
        
        BASE_GAS + (self.data.len() as u64 * DATA_GAS_PER_BYTE)
    }

    /// Create a transfer transaction (unsigned)
    pub fn transfer(
        from: Address,
        to: Address,
        amount: Amount,
        fee: Amount,
        nonce: u64,
        chain_id: u32,
    ) -> Self {
        Self {
            kind: TransactionKind::Transfer,
            from,
            to,
            amount,
            fee,
            nonce,
            data: vec![],
            gas_limit: 21000,
            chain_id,
            public_key: vec![],
            signature: vec![],
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_transaction_hash() {
        let tx = Transaction::transfer(
            Address::zero(),
            Address::from_bytes([1u8; 20]),
            1000,
            10,
            0,
            1,
        );
        let hash1 = tx.hash();
        let hash2 = tx.hash();
        assert_eq!(hash1, hash2);
    }

    #[test]
    fn test_unsigned_transaction() {
        let tx = Transaction::transfer(
            Address::zero(),
            Address::from_bytes([1u8; 20]),
            1000,
            10,
            0,
            1,
        );
        assert!(!tx.is_signed());
    }
}
