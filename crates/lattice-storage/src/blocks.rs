//! Block storage using RocksDB
//!
//! Stores blocks with multiple indexes:
//! - By hash (primary)
//! - By height (index)
//! - Latest block tracking

use crate::error::{Result, StorageError};
use borsh::{BorshDeserialize, BorshSerialize};
use lattice_core::{Block, BlockHeight, Hash};
use parking_lot::RwLock;
use rocksdb::{ColumnFamily, ColumnFamilyDescriptor, Options, DB};
use std::path::Path;
use std::sync::Arc;
use tracing::{debug, info, warn};

/// Column family names
const CF_BLOCKS: &str = "blocks";
const CF_HEIGHT_INDEX: &str = "height_index";
const CF_METADATA: &str = "metadata";

/// Metadata keys
const KEY_LATEST_HASH: &[u8] = b"latest_hash";
const KEY_LATEST_HEIGHT: &[u8] = b"latest_height";
const KEY_GENESIS_HASH: &[u8] = b"genesis_hash";

/// Block storage backed by RocksDB
pub struct BlockStore {
    db: Arc<DB>,
    /// Cache for the latest block to avoid repeated lookups
    latest_cache: RwLock<Option<(Hash, BlockHeight)>>,
}

impl BlockStore {
    /// Open or create a block store at the given path
    pub fn open<P: AsRef<Path>>(path: P) -> Result<Self> {
        let mut opts = Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        let cfs = vec![
            ColumnFamilyDescriptor::new(CF_BLOCKS, Options::default()),
            ColumnFamilyDescriptor::new(CF_HEIGHT_INDEX, Options::default()),
            ColumnFamilyDescriptor::new(CF_METADATA, Options::default()),
        ];

        let db = DB::open_cf_descriptors(&opts, path, cfs)?;
        let store = Self {
            db: Arc::new(db),
            latest_cache: RwLock::new(None),
        };

        // Initialize cache from DB
        store.refresh_cache()?;

        info!("Block store opened successfully");
        Ok(store)
    }

    /// Refresh the latest block cache from DB
    fn refresh_cache(&self) -> Result<()> {
        let cf_meta = self.cf_metadata();

        let latest_hash = self.db.get_cf(cf_meta, KEY_LATEST_HASH)?;
        let latest_height = self.db.get_cf(cf_meta, KEY_LATEST_HEIGHT)?;

        if let (Some(hash_bytes), Some(height_bytes)) = (latest_hash, latest_height) {
            let hash: Hash = hash_bytes
                .as_slice()
                .try_into()
                .map_err(|_| StorageError::Deserialization("invalid hash".into()))?;
            let height = u64::from_le_bytes(
                height_bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| StorageError::Deserialization("invalid height".into()))?,
            );
            *self.latest_cache.write() = Some((hash, height));
        }

        Ok(())
    }

    fn cf_blocks(&self) -> &ColumnFamily {
        self.db.cf_handle(CF_BLOCKS).expect("CF_BLOCKS must exist")
    }

    fn cf_height_index(&self) -> &ColumnFamily {
        self.db
            .cf_handle(CF_HEIGHT_INDEX)
            .expect("CF_HEIGHT_INDEX must exist")
    }

    fn cf_metadata(&self) -> &ColumnFamily {
        self.db
            .cf_handle(CF_METADATA)
            .expect("CF_METADATA must exist")
    }

    /// Store a block
    pub fn put(&self, block: &Block) -> Result<()> {
        let hash = block.hash();
        let height = block.height();
        let encoded =
            borsh::to_vec(block).map_err(|e| StorageError::Serialization(e.to_string()))?;

        let cf_blocks = self.cf_blocks();
        let cf_height = self.cf_height_index();
        let cf_meta = self.cf_metadata();

        // Store block by hash
        self.db.put_cf(cf_blocks, hash, &encoded)?;

        // Store height -> hash index
        self.db.put_cf(cf_height, height.to_le_bytes(), hash)?;

        // Update latest if this is the new tip
        let should_update = {
            let cache = self.latest_cache.read();
            cache.map_or(true, |(_, h)| height > h)
        };

        if should_update {
            self.db.put_cf(cf_meta, KEY_LATEST_HASH, hash)?;
            self.db
                .put_cf(cf_meta, KEY_LATEST_HEIGHT, height.to_le_bytes())?;

            // Store genesis hash if this is block 0
            if height == 0 {
                self.db.put_cf(cf_meta, KEY_GENESIS_HASH, hash)?;
            }

            *self.latest_cache.write() = Some((hash, height));
            debug!(height, ?hash, "Updated latest block");
        }

        Ok(())
    }

    /// Get a block by hash
    pub fn get(&self, hash: &Hash) -> Result<Option<Block>> {
        let cf_blocks = self.cf_blocks();

        match self.db.get_cf(cf_blocks, hash)? {
            Some(bytes) => {
                let block = Block::try_from_slice(&bytes)
                    .map_err(|e| StorageError::Deserialization(e.to_string()))?;
                Ok(Some(block))
            }
            None => Ok(None),
        }
    }

    /// Get a block by height
    pub fn get_by_height(&self, height: BlockHeight) -> Result<Option<Block>> {
        let cf_height = self.cf_height_index();

        // First get hash from height index
        match self.db.get_cf(cf_height, height.to_le_bytes())? {
            Some(hash_bytes) => {
                let hash: Hash = hash_bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| StorageError::Deserialization("invalid hash".into()))?;
                self.get(&hash)
            }
            None => Ok(None),
        }
    }

    /// Get hash at a given height
    pub fn get_hash_by_height(&self, height: BlockHeight) -> Result<Option<Hash>> {
        let cf_height = self.cf_height_index();

        match self.db.get_cf(cf_height, height.to_le_bytes())? {
            Some(hash_bytes) => {
                let hash: Hash = hash_bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| StorageError::Deserialization("invalid hash".into()))?;
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    /// Get the latest block
    pub fn get_latest(&self) -> Result<Option<Block>> {
        let cache = self.latest_cache.read();
        match *cache {
            Some((hash, _)) => self.get(&hash),
            None => Ok(None),
        }
    }

    /// Get the latest block hash and height
    pub fn get_latest_info(&self) -> Option<(Hash, BlockHeight)> {
        *self.latest_cache.read()
    }

    /// Get the latest block height
    pub fn get_latest_height(&self) -> Option<BlockHeight> {
        self.latest_cache.read().map(|(_, h)| h)
    }

    /// Get the genesis block hash
    pub fn get_genesis_hash(&self) -> Result<Option<Hash>> {
        let cf_meta = self.cf_metadata();

        match self.db.get_cf(cf_meta, KEY_GENESIS_HASH)? {
            Some(hash_bytes) => {
                let hash: Hash = hash_bytes
                    .as_slice()
                    .try_into()
                    .map_err(|_| StorageError::Deserialization("invalid hash".into()))?;
                Ok(Some(hash))
            }
            None => Ok(None),
        }
    }

    /// Check if a block exists
    pub fn contains(&self, hash: &Hash) -> Result<bool> {
        let cf_blocks = self.cf_blocks();
        Ok(self.db.get_cf(cf_blocks, hash)?.is_some())
    }

    /// Check if a block exists at height
    pub fn contains_height(&self, height: BlockHeight) -> Result<bool> {
        let cf_height = self.cf_height_index();
        Ok(self.db.get_cf(cf_height, height.to_le_bytes())?.is_some())
    }

    /// Delete a block by hash (used during reorgs)
    pub fn delete(&self, hash: &Hash) -> Result<()> {
        // Get the block first to find its height
        if let Some(block) = self.get(hash)? {
            let height = block.height();
            let cf_blocks = self.cf_blocks();
            let cf_height = self.cf_height_index();

            // Delete block data
            self.db.delete_cf(cf_blocks, hash)?;

            // Delete height index
            self.db.delete_cf(cf_height, height.to_le_bytes())?;

            // Update latest if needed
            let should_update = {
                let cache = self.latest_cache.read();
                cache.map_or(false, |(h, _)| &h == hash)
            };

            if should_update {
                // Find new latest
                if height > 0 {
                    if let Some(prev_hash) = self.get_hash_by_height(height - 1)? {
                        let cf_meta = self.cf_metadata();
                        self.db.put_cf(cf_meta, KEY_LATEST_HASH, prev_hash)?;
                        self.db
                            .put_cf(cf_meta, KEY_LATEST_HEIGHT, (height - 1).to_le_bytes())?;
                        *self.latest_cache.write() = Some((prev_hash, height - 1));
                    }
                } else {
                    // Deleted genesis, clear latest
                    let cf_meta = self.cf_metadata();
                    self.db.delete_cf(cf_meta, KEY_LATEST_HASH)?;
                    self.db.delete_cf(cf_meta, KEY_LATEST_HEIGHT)?;
                    *self.latest_cache.write() = None;
                }
            }

            warn!(height, ?hash, "Deleted block");
        }

        Ok(())
    }

    /// Get a range of blocks by height (inclusive)
    pub fn get_range(&self, start: BlockHeight, end: BlockHeight) -> Result<Vec<Block>> {
        let mut blocks = Vec::new();
        for height in start..=end {
            if let Some(block) = self.get_by_height(height)? {
                blocks.push(block);
            }
        }
        Ok(blocks)
    }

    /// Get total number of blocks stored
    pub fn count(&self) -> Result<u64> {
        match self.get_latest_height() {
            Some(height) => Ok(height + 1),
            None => Ok(0),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::tempdir;

    fn create_test_store() -> (BlockStore, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let store = BlockStore::open(dir.path()).unwrap();
        (store, dir)
    }

    #[test]
    fn test_store_and_retrieve_block() {
        let (store, _dir) = create_test_store();
        let genesis = Block::genesis();

        store.put(&genesis).unwrap();

        let retrieved = store.get(&genesis.hash()).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), genesis);
    }

    #[test]
    fn test_get_by_height() {
        let (store, _dir) = create_test_store();
        let genesis = Block::genesis();

        store.put(&genesis).unwrap();

        let retrieved = store.get_by_height(0).unwrap();
        assert!(retrieved.is_some());
        assert_eq!(retrieved.unwrap(), genesis);
    }

    #[test]
    fn test_latest_block() {
        let (store, _dir) = create_test_store();
        let genesis = Block::genesis();

        store.put(&genesis).unwrap();

        let latest = store.get_latest().unwrap();
        assert!(latest.is_some());
        assert_eq!(latest.unwrap().height(), 0);
    }

    #[test]
    fn test_contains() {
        let (store, _dir) = create_test_store();
        let genesis = Block::genesis();

        assert!(!store.contains(&genesis.hash()).unwrap());

        store.put(&genesis).unwrap();

        assert!(store.contains(&genesis.hash()).unwrap());
    }
}
