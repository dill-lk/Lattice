//! State storage using RocksDB
//!
//! Stores account state with snapshot support for reorgs:
//! - Current state at each address
//! - Historical snapshots keyed by block height
//! - State root computation

use crate::error::{Result, StorageError};
use borsh::BorshDeserialize;
use lattice_core::{Account, Address, BlockHeight, Hash};
use parking_lot::RwLock;
use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, Options, WriteBatch, DB};
use sha3::{Digest, Sha3_256};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info};

/// Column family names
const CF_STATE: &str = "state";
const CF_SNAPSHOTS: &str = "snapshots";
const CF_SNAPSHOT_META: &str = "snapshot_meta";
const CF_CODE: &str = "code";

/// StateStore provides persistent storage for account state
pub struct StateStore {
    db: Arc<DB>,
    /// Current state root cache
    state_root_cache: RwLock<Option<Hash>>,
}

impl StateStore {
    /// Open or create a state store at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_STATE, Options::default()),
            ColumnFamilyDescriptor::new(CF_SNAPSHOTS, Options::default()),
            ColumnFamilyDescriptor::new(CF_SNAPSHOT_META, Options::default()),
            ColumnFamilyDescriptor::new(CF_CODE, Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)?;

        info!("State store opened successfully");
        Ok(Self {
            db: Arc::new(db),
            state_root_cache: RwLock::new(None),
        })
    }

    fn cf_state(&self) -> &ColumnFamily {
        self.db.cf_handle(CF_STATE).expect("CF_STATE must exist")
    }

    fn cf_snapshots(&self) -> &ColumnFamily {
        self.db
            .cf_handle(CF_SNAPSHOTS)
            .expect("CF_SNAPSHOTS must exist")
    }

    fn cf_snapshot_meta(&self) -> &ColumnFamily {
        self.db
            .cf_handle(CF_SNAPSHOT_META)
            .expect("CF_SNAPSHOT_META must exist")
    }

    fn cf_code(&self) -> &ColumnFamily {
        self.db.cf_handle(CF_CODE).expect("CF_CODE must exist")
    }

    /// Get account by address
    pub fn get_account(&self, address: &Address) -> Result<Option<Account>> {
        let cf = self.cf_state();

        match self.db.get_cf(cf, address.as_bytes())? {
            Some(bytes) => {
                let account = Account::try_from_slice(&bytes)
                    .map_err(|e| StorageError::Deserialization(e.to_string()))?;
                Ok(Some(account))
            }
            None => Ok(None),
        }
    }

    /// Set account state
    pub fn set_account(&self, address: &Address, account: &Account) -> Result<()> {
        let cf = self.cf_state();

        if account.is_empty() {
            // Remove empty accounts
            self.db.delete_cf(cf, address.as_bytes())?;
        } else {
            let encoded =
                borsh::to_vec(account).map_err(|e| StorageError::Serialization(e.to_string()))?;
            self.db.put_cf(cf, address.as_bytes(), encoded)?;
        }

        // Invalidate cache
        *self.state_root_cache.write() = None;

        Ok(())
    }

    /// Set multiple accounts atomically
    pub fn set_accounts(&self, accounts: &[(Address, Account)]) -> Result<()> {
        let cf = self.cf_state();
        let mut batch = WriteBatch::default();

        for (address, account) in accounts {
            if account.is_empty() {
                batch.delete_cf(cf, address.as_bytes());
            } else {
                let encoded = borsh::to_vec(account)
                    .map_err(|e| StorageError::Serialization(e.to_string()))?;
                batch.put_cf(cf, address.as_bytes(), encoded);
            }
        }

        self.db.write(batch)?;

        // Invalidate cache
        *self.state_root_cache.write() = None;

        Ok(())
    }

    /// Delete account
    pub fn delete_account(&self, address: &Address) -> Result<()> {
        let cf = self.cf_state();
        self.db.delete_cf(cf, address.as_bytes())?;
        *self.state_root_cache.write() = None;
        Ok(())
    }

    /// Check if account exists
    pub fn account_exists(&self, address: &Address) -> Result<bool> {
        let cf = self.cf_state();
        Ok(self.db.get_cf(cf, address.as_bytes())?.is_some())
    }

    /// Store contract code
    pub fn put_code(&self, code_hash: &Hash, code: &[u8]) -> Result<()> {
        let cf = self.cf_code();
        self.db.put_cf(cf, code_hash, code)?;
        Ok(())
    }

    /// Get contract code by hash
    pub fn get_code(&self, code_hash: &Hash) -> Result<Option<Vec<u8>>> {
        let cf = self.cf_code();
        Ok(self.db.get_cf(cf, code_hash)?)
    }

    /// Compute state root from current state
    pub fn compute_state_root(&self) -> Result<Hash> {
        // Check cache first
        if let Some(cached) = *self.state_root_cache.read() {
            return Ok(cached);
        }

        let cf = self.cf_state();
        let mut hasher = Sha3_256::new();
        let mut count = 0u64;

        // Iterate all accounts in sorted order (RocksDB keys are sorted)
        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, value) = item?;
            hasher.update(&key);
            hasher.update(&value);
            count += 1;
        }

        // Include count in hash for empty state differentiation
        hasher.update(count.to_le_bytes());

        let mut root = [0u8; 32];
        root.copy_from_slice(&hasher.finalize());

        // Cache the result
        *self.state_root_cache.write() = Some(root);

        debug!(?root, accounts = count, "Computed state root");
        Ok(root)
    }

    /// Create a snapshot at a given block height
    pub fn create_snapshot(&self, height: BlockHeight) -> Result<()> {
        let cf_state = self.cf_state();
        let cf_snapshots = self.cf_snapshots();
        let cf_meta = self.cf_snapshot_meta();

        // Collect all current state
        let mut snapshot_data = Vec::new();
        let iter = self.db.iterator_cf(cf_state, rocksdb::IteratorMode::Start);

        for item in iter {
            let (key, value) = item?;
            // Store as (address_len, address, account_len, account)
            let key_len = (key.len() as u32).to_le_bytes();
            let value_len = (value.len() as u32).to_le_bytes();
            snapshot_data.extend_from_slice(&key_len);
            snapshot_data.extend_from_slice(&key);
            snapshot_data.extend_from_slice(&value_len);
            snapshot_data.extend_from_slice(&value);
        }

        // Store snapshot
        let snapshot_key = height.to_le_bytes();
        self.db.put_cf(cf_snapshots, snapshot_key, &snapshot_data)?;

        // Store metadata (state root at this height)
        let state_root = self.compute_state_root()?;
        self.db.put_cf(cf_meta, snapshot_key, state_root)?;

        info!(height, "Created state snapshot");
        Ok(())
    }

    /// Rollback to a snapshot at a given block height
    pub fn rollback_to_snapshot(&self, height: BlockHeight) -> Result<()> {
        let cf_state = self.cf_state();
        let cf_snapshots = self.cf_snapshots();
        let cf_meta = self.cf_snapshot_meta();

        let snapshot_key = height.to_le_bytes();

        // Get snapshot data
        let snapshot_data = self
            .db
            .get_cf(cf_snapshots, snapshot_key)?
            .ok_or(StorageError::SnapshotNotFound(height))?;

        // Clear current state
        let iter = self.db.iterator_cf(cf_state, rocksdb::IteratorMode::Start);
        let mut batch = WriteBatch::default();
        for item in iter {
            let (key, _) = item?;
            batch.delete_cf(cf_state, &key);
        }
        self.db.write(batch)?;

        // Restore from snapshot
        let mut batch = WriteBatch::default();
        let mut cursor = 0;

        while cursor < snapshot_data.len() {
            // Read key length
            let key_len =
                u32::from_le_bytes(snapshot_data[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;

            // Read key
            let key = &snapshot_data[cursor..cursor + key_len];
            cursor += key_len;

            // Read value length
            let value_len =
                u32::from_le_bytes(snapshot_data[cursor..cursor + 4].try_into().unwrap()) as usize;
            cursor += 4;

            // Read value
            let value = &snapshot_data[cursor..cursor + value_len];
            cursor += value_len;

            batch.put_cf(cf_state, key, value);
        }

        self.db.write(batch)?;

        // Restore cached state root from metadata
        if let Some(root_bytes) = self.db.get_cf(cf_meta, snapshot_key)? {
            let root: Hash = root_bytes
                .as_slice()
                .try_into()
                .map_err(|_| StorageError::Deserialization("invalid state root".into()))?;
            *self.state_root_cache.write() = Some(root);
        } else {
            *self.state_root_cache.write() = None;
        }

        info!(height, "Rolled back to snapshot");
        Ok(())
    }

    /// Delete snapshots older than a given height
    pub fn prune_snapshots(&self, keep_after: BlockHeight) -> Result<u64> {
        let cf_snapshots = self.cf_snapshots();
        let cf_meta = self.cf_snapshot_meta();

        let mut batch = WriteBatch::default();
        let mut pruned = 0u64;

        let iter = self.db.iterator_cf(cf_snapshots, rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, _) = item?;
            if key.len() == 8 {
                let height = u64::from_le_bytes(key.as_ref().try_into().unwrap());
                if height < keep_after {
                    batch.delete_cf(cf_snapshots, &key);
                    batch.delete_cf(cf_meta, &key);
                    pruned += 1;
                }
            }
        }

        self.db.write(batch)?;

        if pruned > 0 {
            info!(pruned, keep_after, "Pruned old snapshots");
        }

        Ok(pruned)
    }

    /// Check if snapshot exists at height
    pub fn has_snapshot(&self, height: BlockHeight) -> Result<bool> {
        let cf_snapshots = self.cf_snapshots();
        Ok(self.db.get_cf(cf_snapshots, height.to_le_bytes())?.is_some())
    }

    /// Get state root from snapshot metadata
    pub fn get_snapshot_root(&self, height: BlockHeight) -> Result<Option<Hash>> {
        let cf_meta = self.cf_snapshot_meta();

        match self.db.get_cf(cf_meta, height.to_le_bytes())? {
            Some(bytes) => {
                let hash: Hash = bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| StorageError::Deserialization("invalid hash".into()))?;
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    /// Get all account addresses (for debugging/inspection)
    pub fn list_accounts(&self) -> Result<Vec<Address>> {
        let cf = self.cf_state();
        let mut addresses = Vec::new();

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        for item in iter {
            let (key, _) = item?;
            if key.len() == 20 {
                let addr_bytes: [u8; 20] = key.as_ref().try_into().unwrap();
                addresses.push(Address::from_bytes(addr_bytes));
            }
        }

        Ok(addresses)
    }

    /// Get total account count
    pub fn account_count(&self) -> Result<u64> {
        let cf = self.cf_state();
        let mut count = 0u64;

        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);
        for item in iter {
            let _ = item?;
            count += 1;
        }

        Ok(count)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_store() -> (StateStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let store = StateStore::open(dir.path()).unwrap();
        (store, dir)
    }

    #[test]
    fn test_account_storage() {
        let (store, _dir) = create_test_store();
        let address = Address::from_bytes([1u8; 20]);
        let account = Account::with_balance(1000);

        store.set_account(&address, &account).unwrap();

        let retrieved = store.get_account(&address).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().balance, 1000);
    }

    #[test]
    fn test_empty_account_deletion() {
        let (store, _dir) = create_test_store();
        let address = Address::from_bytes([1u8; 20]);
        let account = Account::with_balance(1000);

        store.set_account(&address, &account).unwrap();
        assert!(store.account_exists(&address).unwrap());

        // Set empty account should delete
        store.set_account(&address, &Account::new()).unwrap();
        assert!(!store.account_exists(&address).unwrap());
    }

    #[test]
    fn test_state_root_computation() {
        let (store, _dir) = create_test_store();

        let root1 = store.compute_state_root().unwrap();

        let address = Address::from_bytes([1u8; 20]);
        store
            .set_account(&address, &Account::with_balance(100))
            .unwrap();

        let root2 = store.compute_state_root().unwrap();
        assert_ne!(root1, root2);

        // Same state should give same root
        let root3 = store.compute_state_root().unwrap();
        assert_eq!(root2, root3);
    }

    #[test]
    fn test_snapshot_and_rollback() {
        let (store, _dir) = create_test_store();
        let address = Address::from_bytes([1u8; 20]);

        // Initial state
        store
            .set_account(&address, &Account::with_balance(100))
            .unwrap();
        store.create_snapshot(1).unwrap();

        // Modify state
        store
            .set_account(&address, &Account::with_balance(200))
            .unwrap();
        assert_eq!(store.get_account(&address).unwrap().unwrap().balance, 200);

        // Rollback
        store.rollback_to_snapshot(1).unwrap();
        assert_eq!(store.get_account(&address).unwrap().unwrap().balance, 100);
    }

    #[test]
    fn test_code_storage() {
        let (store, _dir) = create_test_store();
        let code = b"contract bytecode";
        let code_hash = {
            use sha3::Digest;
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&Sha3_256::digest(code));
            hash
        };

        store.put_code(&code_hash, code).unwrap();

        let retrieved = store.get_code(&code_hash).unwrap();
        assert_eq!(retrieved.unwrap(), code);
    }
}
