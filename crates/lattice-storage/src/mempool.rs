//! Mempool (transaction pool) storage
//!
//! Manages pending transactions with:
//! - Fast lookup by hash
//! - Ordering by fee for block production
//! - Eviction policy for size limits
//! - Persistence across restarts

use crate::error::{Result, StorageError};
use borsh::BorshDeserialize;
use lattice_core::{Address, Amount, Hash, Transaction};
use parking_lot::RwLock;
use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, Options, WriteBatch, DB};
use std::collections::{BinaryHeap, HashMap, HashSet};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Column family names
const CF_TRANSACTIONS: &str = "transactions";
const CF_METADATA: &str = "mempool_meta";

/// Default mempool limits
const DEFAULT_MAX_SIZE: usize = 10_000;
const DEFAULT_MAX_SIZE_BYTES: usize = 32 * 1024 * 1024; // 32 MB

/// Transaction entry with metadata for sorting
#[derive(Debug, Clone)]
struct TxEntry {
    /// Transaction hash
    hash: Hash,
    /// Fee (for priority sorting)
    fee: Amount,
    /// Fee per gas (for priority sorting)
    fee_per_gas: Amount,
    /// Transaction size in bytes
    size: usize,
    /// Insertion timestamp (for FIFO within same fee tier)
    inserted_at: u64,
}

impl PartialEq for TxEntry {
    fn eq(&self, other: &Self) -> bool {
        self.hash == other.hash
    }
}

impl Eq for TxEntry {}

impl PartialOrd for TxEntry {
    fn partial_cmp(&self, other: &Self) -> Option<std::cmp::Ordering> {
        Some(self.cmp(other))
    }
}

impl Ord for TxEntry {
    fn cmp(&self, other: &Self) -> std::cmp::Ordering {
        // Higher fee per gas = higher priority
        // For equal fees, prefer older transactions (FIFO)
        self.fee_per_gas
            .cmp(&other.fee_per_gas)
            .then_with(|| other.inserted_at.cmp(&self.inserted_at))
    }
}

/// Mempool configuration
#[derive(Debug, Clone)]
pub struct MempoolConfig {
    /// Maximum number of transactions
    pub max_size: usize,
    /// Maximum total size in bytes
    pub max_size_bytes: usize,
    /// Minimum fee per gas to accept
    pub min_fee_per_gas: Amount,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_size: DEFAULT_MAX_SIZE,
            max_size_bytes: DEFAULT_MAX_SIZE_BYTES,
            min_fee_per_gas: 1,
        }
    }
}

/// Mempool store for pending transactions
pub struct MempoolStore {
    db: Arc<DB>,
    /// In-memory index for fast access
    index: RwLock<HashMap<Hash, TxEntry>>,
    /// Priority queue for block production
    priority_queue: RwLock<BinaryHeap<TxEntry>>,
    /// Transactions by sender (for nonce tracking)
    by_sender: RwLock<HashMap<Address, HashSet<Hash>>>,
    /// Current total size in bytes
    total_size: RwLock<usize>,
    /// Configuration
    config: MempoolConfig,
    /// Insertion counter for ordering
    insertion_counter: RwLock<u64>,
}

impl MempoolStore {
    /// Open or create a mempool store at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        Self::open_with_config(path, MempoolConfig::default())
    }

    /// Open with custom configuration
    pub fn open_with_config<P: AsRef<Path>>(path: P, config: MempoolConfig) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_TRANSACTIONS, Options::default()),
            ColumnFamilyDescriptor::new(CF_METADATA, Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)?;

        let store = Self {
            db: Arc::new(db),
            index: RwLock::new(HashMap::new()),
            priority_queue: RwLock::new(BinaryHeap::new()),
            by_sender: RwLock::new(HashMap::new()),
            total_size: RwLock::new(0),
            config,
            insertion_counter: RwLock::new(0),
        };

        // Load existing transactions from disk
        store.load_from_disk()?;

        info!(
            count = store.len(),
            size = *store.total_size.read(),
            "Mempool opened"
        );
        Ok(store)
    }

    fn cf_transactions(&self) -> &ColumnFamily {
        self.db
            .cf_handle(CF_TRANSACTIONS)
            .expect("CF_TRANSACTIONS must exist")
    }

    /// Load transactions from disk into memory indexes
    fn load_from_disk(&self) -> Result<()> {
        let cf = self.cf_transactions();
        let iter = self.db.iterator_cf(cf, rocksdb::IteratorMode::Start);

        let mut index = self.index.write();
        let mut priority_queue = self.priority_queue.write();
        let mut by_sender = self.by_sender.write();
        let mut total_size = self.total_size.write();
        let mut counter = self.insertion_counter.write();

        for item in iter {
            let (key, value) = item?;

            let tx = Transaction::try_from_slice(&value)
                .map_err(|e| StorageError::Deserialization(e.to_string()))?;

            let hash: Hash = key
                .as_ref()
                .try_into()
                .map_err(|_| StorageError::Deserialization("invalid hash".into()))?;

            let size = value.len();
            let fee_per_gas = if tx.gas_limit > 0 {
                tx.fee / tx.gas_limit as u128
            } else {
                tx.fee
            };

            let entry = TxEntry {
                hash,
                fee: tx.fee,
                fee_per_gas,
                size,
                inserted_at: *counter,
            };
            *counter += 1;

            index.insert(hash, entry.clone());
            priority_queue.push(entry);
            by_sender
                .entry(tx.from.clone())
                .or_default()
                .insert(hash);
            *total_size += size;
        }

        Ok(())
    }

    /// Add a transaction to the mempool
    pub fn add(&self, tx: &Transaction) -> Result<()> {
        let hash = tx.hash();

        // Check for duplicates
        if self.index.read().contains_key(&hash) {
            return Err(StorageError::DuplicateTransaction);
        }

        // Calculate fee per gas
        let fee_per_gas = if tx.gas_limit > 0 {
            tx.fee / tx.gas_limit as u128
        } else {
            tx.fee
        };

        // Check minimum fee
        if fee_per_gas < self.config.min_fee_per_gas {
            return Err(StorageError::Serialization(format!(
                "fee too low: {} < {}",
                fee_per_gas, self.config.min_fee_per_gas
            )));
        }

        // Serialize transaction
        let encoded =
            borsh::to_vec(tx).map_err(|e| StorageError::Serialization(e.to_string()))?;
        let size = encoded.len();

        // Check size limits and evict if needed
        self.ensure_capacity(1, size)?;

        // Store to disk
        let cf = self.cf_transactions();
        self.db.put_cf(cf, hash, &encoded)?;

        // Update in-memory indexes
        let inserted_at = {
            let mut counter = self.insertion_counter.write();
            let val = *counter;
            *counter += 1;
            val
        };

        let entry = TxEntry {
            hash,
            fee: tx.fee,
            fee_per_gas,
            size,
            inserted_at,
        };

        self.index.write().insert(hash, entry.clone());
        self.priority_queue.write().push(entry);
        self.by_sender
            .write()
            .entry(tx.from.clone())
            .or_default()
            .insert(hash);
        *self.total_size.write() += size;

        debug!(?hash, fee = tx.fee, "Added transaction to mempool");
        Ok(())
    }

    /// Ensure capacity for new transactions, evicting low-priority ones if needed
    fn ensure_capacity(&self, count: usize, size: usize) -> Result<()> {
        let current_count = self.index.read().len();
        let current_size = *self.total_size.read();

        // Check if we need to evict
        if current_count + count > self.config.max_size
            || current_size + size > self.config.max_size_bytes
        {
            // Evict lowest priority transactions
            let to_evict = std::cmp::max(
                (current_count + count).saturating_sub(self.config.max_size),
                1,
            );

            let evicted = self.evict_lowest_priority(to_evict)?;
            if evicted == 0 && current_count >= self.config.max_size {
                return Err(StorageError::MempoolFull);
            }
        }

        Ok(())
    }

    /// Evict lowest priority transactions
    fn evict_lowest_priority(&self, count: usize) -> Result<usize> {
        // Get lowest priority entries
        let mut to_remove = Vec::new();
        {
            let index = self.index.read();
            let mut entries: Vec<_> = index.values().cloned().collect();
            // Sort by priority (ascending, so lowest priority first)
            entries.sort();

            for entry in entries.into_iter().take(count) {
                to_remove.push(entry.hash);
            }
        }

        // Remove them
        for hash in &to_remove {
            self.remove(hash)?;
        }

        if !to_remove.is_empty() {
            warn!(count = to_remove.len(), "Evicted transactions from mempool");
        }

        Ok(to_remove.len())
    }

    /// Remove a transaction by hash
    pub fn remove(&self, hash: &Hash) -> Result<Option<Transaction>> {
        // Get transaction first
        let tx = self.get(hash)?;

        if let Some(ref tx) = tx {
            let cf = self.cf_transactions();

            // Remove from disk
            self.db.delete_cf(cf, hash)?;

            // Remove from indexes
            if let Some(entry) = self.index.write().remove(hash) {
                *self.total_size.write() -= entry.size;
            }

            // Remove from sender index
            let mut by_sender = self.by_sender.write();
            if let Some(hashes) = by_sender.get_mut(&tx.from) {
                hashes.remove(hash);
                if hashes.is_empty() {
                    by_sender.remove(&tx.from);
                }
            }

            // Note: We don't remove from priority_queue for efficiency
            // Invalid entries will be filtered when popping

            debug!(?hash, "Removed transaction from mempool");
        }

        Ok(tx)
    }

    /// Remove multiple transactions atomically
    pub fn remove_batch(&self, hashes: &[Hash]) -> Result<Vec<Transaction>> {
        let cf = self.cf_transactions();
        let mut batch = WriteBatch::default();
        let mut removed = Vec::new();

        for hash in hashes {
            if let Some(tx) = self.get(hash)? {
                batch.delete_cf(cf, hash);

                // Remove from indexes
                if let Some(entry) = self.index.write().remove(hash) {
                    *self.total_size.write() -= entry.size;
                }

                // Remove from sender index
                let mut by_sender = self.by_sender.write();
                if let Some(sender_hashes) = by_sender.get_mut(&tx.from) {
                    sender_hashes.remove(hash);
                    if sender_hashes.is_empty() {
                        by_sender.remove(&tx.from);
                    }
                }

                removed.push(tx);
            }
        }

        self.db.write(batch)?;

        debug!(count = removed.len(), "Removed batch from mempool");
        Ok(removed)
    }

    /// Get a transaction by hash
    pub fn get(&self, hash: &Hash) -> Result<Option<Transaction>> {
        let cf = self.cf_transactions();

        match self.db.get_cf(cf, hash)? {
            Some(bytes) => {
                let tx = Transaction::try_from_slice(&bytes)
                    .map_err(|e| StorageError::Deserialization(e.to_string()))?;
                Ok(Some(tx))
            }
            None => Ok(None),
        }
    }

    /// Check if a transaction exists
    pub fn contains(&self, hash: &Hash) -> bool {
        self.index.read().contains_key(hash)
    }

    /// Get transactions sorted by fee (highest first) for block production
    pub fn get_sorted_by_fee(&self, limit: usize) -> Result<Vec<Transaction>> {
        let index = self.index.read();
        let mut entries: Vec<_> = index.values().cloned().collect();

        // Sort by priority (descending)
        entries.sort_by(|a, b| b.cmp(a));

        let mut result = Vec::with_capacity(limit.min(entries.len()));
        for entry in entries.into_iter().take(limit) {
            if let Some(tx) = self.get(&entry.hash)? {
                result.push(tx);
            }
        }

        Ok(result)
    }

    /// Get transactions for a specific sender
    pub fn get_by_sender(&self, sender: &Address) -> Result<Vec<Transaction>> {
        let hashes: Vec<Hash> = self
            .by_sender
            .read()
            .get(sender)
            .map(|h| h.iter().cloned().collect())
            .unwrap_or_default();

        let mut result = Vec::with_capacity(hashes.len());
        for hash in hashes {
            if let Some(tx) = self.get(&hash)? {
                result.push(tx);
            }
        }

        // Sort by nonce
        result.sort_by_key(|tx| tx.nonce);
        Ok(result)
    }

    /// Get number of transactions in mempool
    pub fn len(&self) -> usize {
        self.index.read().len()
    }

    /// Check if mempool is empty
    pub fn is_empty(&self) -> bool {
        self.index.read().is_empty()
    }

    /// Get total size in bytes
    pub fn size_bytes(&self) -> usize {
        *self.total_size.read()
    }

    /// Clear all transactions
    pub fn clear(&self) -> Result<()> {
        let cf = self.cf_transactions();

        // Delete all from disk
        let hashes: Vec<Hash> = self.index.read().keys().cloned().collect();
        for hash in hashes {
            self.db.delete_cf(cf, hash)?;
        }

        // Clear indexes
        self.index.write().clear();
        self.priority_queue.write().clear();
        self.by_sender.write().clear();
        *self.total_size.write() = 0;

        info!("Mempool cleared");
        Ok(())
    }

    /// Get all transaction hashes
    pub fn get_all_hashes(&self) -> Vec<Hash> {
        self.index.read().keys().cloned().collect()
    }

    /// Get mempool statistics
    pub fn stats(&self) -> MempoolStats {
        let index = self.index.read();

        let mut total_fees = 0u128;
        let mut min_fee = Amount::MAX;
        let mut max_fee = 0u128;

        for entry in index.values() {
            total_fees += entry.fee;
            min_fee = min_fee.min(entry.fee);
            max_fee = max_fee.max(entry.fee);
        }

        let count = index.len();

        MempoolStats {
            count,
            size_bytes: *self.total_size.read(),
            total_fees,
            avg_fee: if count > 0 {
                total_fees / count as u128
            } else {
                0
            },
            min_fee: if count > 0 { min_fee } else { 0 },
            max_fee,
        }
    }
}

/// Mempool statistics
#[derive(Debug, Clone)]
pub struct MempoolStats {
    /// Number of transactions
    pub count: usize,
    /// Total size in bytes
    pub size_bytes: usize,
    /// Total fees
    pub total_fees: Amount,
    /// Average fee
    pub avg_fee: Amount,
    /// Minimum fee
    pub min_fee: Amount,
    /// Maximum fee
    pub max_fee: Amount,
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_store() -> (MempoolStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let config = MempoolConfig {
            min_fee_per_gas: 0,
            ..Default::default()
        };
        let store = MempoolStore::open_with_config(dir.path(), config).unwrap();
        (store, dir)
    }

    fn create_test_tx(nonce: u64, fee: Amount) -> Transaction {
        Transaction {
            kind: lattice_core::TransactionKind::Transfer,
            from: Address::from_bytes([1u8; 20]),
            to: Address::from_bytes([2u8; 20]),
            amount: 100,
            fee,
            nonce,
            data: vec![],
            gas_limit: 21000,
            chain_id: 1,
            public_key: vec![],
            signature: vec![],
        }
    }

    #[test]
    fn test_add_and_get() {
        let (store, _dir) = create_test_store();
        let tx = create_test_tx(0, 100);

        store.add(&tx).unwrap();

        let retrieved = store.get(&tx.hash()).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap().fee, 100);
    }

    #[test]
    fn test_duplicate_rejection() {
        let (store, _dir) = create_test_store();
        let tx = create_test_tx(0, 100);

        store.add(&tx).unwrap();
        let result = store.add(&tx);
        assert!(result.is_err());
    }

    #[test]
    fn test_remove() {
        let (store, _dir) = create_test_store();
        let tx = create_test_tx(0, 100);
        let hash = tx.hash();

        store.add(&tx).unwrap();
        assert!(store.contains(&hash));

        store.remove(&hash).unwrap();
        assert!(!store.contains(&hash));
    }

    #[test]
    fn test_sorted_by_fee() {
        let (store, _dir) = create_test_store();

        store.add(&create_test_tx(0, 100)).unwrap();
        store.add(&create_test_tx(1, 300)).unwrap();
        store.add(&create_test_tx(2, 200)).unwrap();

        let sorted = store.get_sorted_by_fee(10).unwrap();
        assert_eq!(sorted.len(), 3);
        assert!(sorted[0].fee >= sorted[1].fee);
        assert!(sorted[1].fee >= sorted[2].fee);
    }

    #[test]
    fn test_get_by_sender() {
        let (store, _dir) = create_test_store();
        let sender = Address::from_bytes([1u8; 20]);

        store.add(&create_test_tx(0, 100)).unwrap();
        store.add(&create_test_tx(1, 200)).unwrap();

        let txs = store.get_by_sender(&sender).unwrap();
        assert_eq!(txs.len(), 2);
        // Should be sorted by nonce
        assert_eq!(txs[0].nonce, 0);
        assert_eq!(txs[1].nonce, 1);
    }

    #[test]
    fn test_stats() {
        let (store, _dir) = create_test_store();

        store.add(&create_test_tx(0, 100)).unwrap();
        store.add(&create_test_tx(1, 200)).unwrap();

        let stats = store.stats();
        assert_eq!(stats.count, 2);
        assert_eq!(stats.total_fees, 300);
        assert_eq!(stats.min_fee, 100);
        assert_eq!(stats.max_fee, 200);
    }
}
