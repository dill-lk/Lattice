//! Account state for Lattice blockchain

use crate::{Address, Amount, Hash};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

/// Account state
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Account {
    /// Account balance
    pub balance: Amount,
    /// Transaction count (nonce)
    pub nonce: u64,
    /// Code hash (for contract accounts, empty for EOA)
    pub code_hash: Hash,
    /// Storage root (Merkle root of account storage)
    pub storage_root: Hash,
}

impl Account {
    /// Create a new empty account
    pub fn new() -> Self {
        Self {
            balance: 0,
            nonce: 0,
            code_hash: [0u8; 32],
            storage_root: [0u8; 32],
        }
    }

    /// Create account with initial balance
    pub fn with_balance(balance: Amount) -> Self {
        Self {
            balance,
            nonce: 0,
            code_hash: [0u8; 32],
            storage_root: [0u8; 32],
        }
    }

    /// Check if this is a contract account
    pub fn is_contract(&self) -> bool {
        self.code_hash != [0u8; 32]
    }

    /// Check if account is empty (can be pruned)
    pub fn is_empty(&self) -> bool {
        self.balance == 0 && self.nonce == 0 && !self.is_contract()
    }
}

impl Default for Account {
    fn default() -> Self {
        Self::new()
    }
}

/// World state - mapping of addresses to accounts
#[derive(Debug, Clone, Default)]
pub struct State {
    /// Account states
    accounts: HashMap<Address, Account>,
}

impl State {
    /// Create empty state
    pub fn new() -> Self {
        Self {
            accounts: HashMap::new(),
        }
    }

    /// Get account (returns empty account if not exists)
    pub fn get_account(&self, address: &Address) -> Account {
        self.accounts.get(address).cloned().unwrap_or_default()
    }

    /// Get mutable account reference
    pub fn get_account_mut(&mut self, address: &Address) -> &mut Account {
        self.accounts.entry(address.clone()).or_default()
    }

    /// Set account state
    pub fn set_account(&mut self, address: Address, account: Account) {
        if account.is_empty() {
            self.accounts.remove(&address);
        } else {
            self.accounts.insert(address, account);
        }
    }

    /// Get balance of an address
    pub fn balance(&self, address: &Address) -> Amount {
        self.get_account(address).balance
    }

    /// Get nonce of an address
    pub fn nonce(&self, address: &Address) -> u64 {
        self.get_account(address).nonce
    }

    /// Check if address exists
    pub fn exists(&self, address: &Address) -> bool {
        self.accounts.contains_key(address)
    }

    /// Transfer value between accounts
    pub fn transfer(
        &mut self,
        from: &Address,
        to: &Address,
        amount: Amount,
    ) -> Result<(), StateError> {
        let from_balance = self.balance(from);
        if from_balance < amount {
            return Err(StateError::InsufficientBalance);
        }

        {
            let from_account = self.get_account_mut(from);
            from_account.balance -= amount;
        }

        {
            let to_account = self.get_account_mut(to);
            to_account.balance += amount;
        }

        Ok(())
    }

    /// Increment nonce for an address
    pub fn increment_nonce(&mut self, address: &Address) {
        let account = self.get_account_mut(address);
        account.nonce += 1;
    }

    /// Calculate state root (simplified - real impl would use Merkle Patricia Trie)
    pub fn root(&self) -> Hash {
        use sha3::{Digest, Sha3_256};
        
        let mut hasher = Sha3_256::new();
        let mut addrs: Vec<_> = self.accounts.keys().collect();
        addrs.sort_by_key(|a| a.as_bytes());
        
        for addr in addrs {
            let account = &self.accounts[addr];
            hasher.update(addr.as_bytes());
            hasher.update(account.balance.to_le_bytes());
            hasher.update(account.nonce.to_le_bytes());
            hasher.update(account.code_hash);
        }
        
        let mut hash = [0u8; 32];
        hash.copy_from_slice(&hasher.finalize());
        hash
    }
}

#[derive(Debug, thiserror::Error)]
pub enum StateError {
    #[error("insufficient balance")]
    InsufficientBalance,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_account_default() {
        let account = Account::new();
        assert_eq!(account.balance, 0);
        assert_eq!(account.nonce, 0);
        assert!(account.is_empty());
    }

    #[test]
    fn test_state_transfer() {
        let mut state = State::new();
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);
        
        state.set_account(from.clone(), Account::with_balance(1000));
        
        state.transfer(&from, &to, 400).unwrap();
        
        assert_eq!(state.balance(&from), 600);
        assert_eq!(state.balance(&to), 400);
    }

    #[test]
    fn test_insufficient_balance() {
        let mut state = State::new();
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);
        
        state.set_account(from.clone(), Account::with_balance(100));
        
        let result = state.transfer(&from, &to, 500);
        assert!(result.is_err());
    }
}
