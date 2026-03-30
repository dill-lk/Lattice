//! RPC method handlers

use crate::error::{Result, RpcError};
use crate::types::{
    BlockNumber, BlockTag, CallRequest, RpcBlock, RpcTransaction, TransactionReceipt,
};
use lattice_consensus::{verify_pow, PoWConfig};
use lattice_core::{Address, Amount, Block, BlockHeader, BlockHeight, Hash, Transaction};
use parking_lot::RwLock;
use serde_json::{json, Value};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use tracing::{debug, warn};

const MAINNET_GENESIS_DIFFICULTY: u64 = 20_000;
const MAINNET_DYNAMIC_DIFFICULTY_START_HEIGHT: u64 = 10;
const MAINNET_DYNAMIC_ADJUSTMENT_INTERVAL: u64 = 10;
const MAINNET_TARGET_BLOCK_TIME_MS: u64 = 15_000;

/// Blockchain state for RPC handlers
pub struct ChainState {
    /// Blocks indexed by height
    pub blocks_by_height: HashMap<BlockHeight, Block>,
    /// Blocks indexed by hash
    pub blocks_by_hash: HashMap<Hash, Block>,
    /// Transactions indexed by hash
    pub transactions: HashMap<Hash, (Transaction, Option<Hash>)>, // (tx, block_hash)
    /// Account balances
    pub balances: HashMap<Address, Amount>,
    /// Current block height
    pub height: BlockHeight,
    /// Pending transactions (mempool)
    pub pending_txs: Vec<Transaction>,
    /// Pending mining work templates indexed by work_id
    pub pending_work: HashMap<String, Block>,
    /// PoW configuration for block verification
    pub pow_config: PoWConfig,
}

impl Default for ChainState {
    fn default() -> Self {
        Self::new()
    }
}

impl ChainState {
    pub fn new() -> Self {
        let genesis = Block::genesis();
        let genesis_hash = genesis.hash();

        let mut blocks_by_height = HashMap::new();
        let mut blocks_by_hash = HashMap::new();

        blocks_by_height.insert(0, genesis.clone());
        blocks_by_hash.insert(genesis_hash, genesis);

        Self {
            blocks_by_height,
            blocks_by_hash,
            transactions: HashMap::new(),
            balances: HashMap::new(),
            height: 0,
            pending_txs: Vec::new(),
            pending_work: HashMap::new(),
            pow_config: PoWConfig::default(),
        }
    }

    /// Create a new ChainState with a specific PoW configuration
    pub fn with_pow_config(pow_config: PoWConfig) -> Self {
        let mut state = Self::new();
        state.pow_config = pow_config;
        state
    }
}

/// RPC handlers for Lattice blockchain
pub struct RpcHandlers {
    state: Arc<RwLock<ChainState>>,
    /// Monotonic counter for generating unique work IDs
    work_counter: AtomicU64,
}

impl Default for RpcHandlers {
    fn default() -> Self {
        Self::new()
    }
}

impl RpcHandlers {
    /// Create new RPC handlers with default state
    pub fn new() -> Self {
        Self {
            state: Arc::new(RwLock::new(ChainState::new())),
            work_counter: AtomicU64::new(0),
        }
    }

    /// Create RPC handlers with shared state
    pub fn with_state(state: Arc<RwLock<ChainState>>) -> Self {
        Self {
            state,
            work_counter: AtomicU64::new(0),
        }
    }

    /// Get shared state
    pub fn state(&self) -> Arc<RwLock<ChainState>> {
        Arc::clone(&self.state)
    }

    /// Route RPC method calls
    pub fn handle(&self, method: &str, params: Value) -> Result<Value> {
        debug!(method = %method, "Handling RPC method");

        match method {
            "lat_blockNumber" => self.lat_block_number(),
            "lat_getBlockByNumber" => self.lat_get_block_by_number(params),
            "lat_getBlockByHash" => self.lat_get_block_by_hash(params),
            "lat_getTransactionByHash" => self.lat_get_transaction_by_hash(params),
            "lat_getBalance" => self.lat_get_balance(params),
            "lat_sendRawTransaction" => self.lat_send_raw_transaction(params),
            "lat_getTransactionReceipt" => self.lat_get_transaction_receipt(params),
            "lat_call" => self.lat_call(params),
            "lat_estimateGas" => self.lat_estimate_gas(params),
            "lat_getWork" => self.lat_get_work(params),
            "lat_submitWork" => self.lat_submit_work(params),
            _ => {
                warn!(method = %method, "Unknown RPC method");
                Err(RpcError::method_not_found())
            }
        }
    }

    /// lat_blockNumber - Get the latest block number
    pub fn lat_block_number(&self) -> Result<Value> {
        let state = self.state.read();
        Ok(json!(format!("0x{:x}", state.height)))
    }

    /// lat_getBlockByNumber - Get block by height
    pub fn lat_get_block_by_number(&self, params: Value) -> Result<Value> {
        let params: Vec<Value> = serde_json::from_value(params)
            .map_err(|_| RpcError::invalid_params("Expected array of parameters"))?;

        if params.is_empty() {
            return Err(RpcError::invalid_params("Missing block number parameter"));
        }

        let block_num: BlockNumber = serde_json::from_value(params[0].clone())
            .map_err(|_| RpcError::invalid_params("Invalid block number"))?;

        let include_txs = params.get(1).and_then(|v| v.as_bool()).unwrap_or(false);

        let state = self.state.read();
        let height = match block_num {
            BlockNumber::Number(n) => n,
            BlockNumber::Tag(BlockTag::Latest) => state.height,
            BlockNumber::Tag(BlockTag::Earliest) => 0,
            BlockNumber::Tag(BlockTag::Pending) => state.height,
        };

        match state.blocks_by_height.get(&height) {
            Some(block) => Ok(json!(self.block_to_rpc(block, include_txs))),
            None => Err(RpcError::block_not_found()),
        }
    }

    /// lat_getBlockByHash - Get block by hash
    pub fn lat_get_block_by_hash(&self, params: Value) -> Result<Value> {
        let params: Vec<Value> = serde_json::from_value(params)
            .map_err(|_| RpcError::invalid_params("Expected array of parameters"))?;

        if params.is_empty() {
            return Err(RpcError::invalid_params("Missing block hash parameter"));
        }

        let hash_str = params[0]
            .as_str()
            .ok_or_else(|| RpcError::invalid_params("Block hash must be a string"))?;

        let hash = parse_hash(hash_str)?;
        let include_txs = params.get(1).and_then(|v| v.as_bool()).unwrap_or(false);

        let state = self.state.read();
        match state.blocks_by_hash.get(&hash) {
            Some(block) => Ok(json!(self.block_to_rpc(block, include_txs))),
            None => Err(RpcError::block_not_found()),
        }
    }

    /// lat_getTransactionByHash - Get transaction by hash
    pub fn lat_get_transaction_by_hash(&self, params: Value) -> Result<Value> {
        let params: Vec<Value> = serde_json::from_value(params)
            .map_err(|_| RpcError::invalid_params("Expected array of parameters"))?;

        if params.is_empty() {
            return Err(RpcError::invalid_params(
                "Missing transaction hash parameter",
            ));
        }

        let hash_str = params[0]
            .as_str()
            .ok_or_else(|| RpcError::invalid_params("Transaction hash must be a string"))?;

        let hash = parse_hash(hash_str)?;

        let state = self.state.read();
        match state.transactions.get(&hash) {
            Some((tx, block_hash)) => {
                let rpc_tx = self.transaction_to_rpc(tx, block_hash.as_ref());
                Ok(json!(rpc_tx))
            }
            None => Ok(Value::Null),
        }
    }

    /// lat_getBalance - Get account balance
    pub fn lat_get_balance(&self, params: Value) -> Result<Value> {
        let params: Vec<Value> = serde_json::from_value(params)
            .map_err(|_| RpcError::invalid_params("Expected array of parameters"))?;

        if params.is_empty() {
            return Err(RpcError::invalid_params("Missing address parameter"));
        }

        let addr_str = params[0]
            .as_str()
            .ok_or_else(|| RpcError::invalid_params("Address must be a string"))?;

        let address = Address::from_base58(addr_str)
            .map_err(|_| RpcError::invalid_params("Invalid address format"))?;

        let state = self.state.read();
        let balance = state.balances.get(&address).copied().unwrap_or(0);

        Ok(json!(format!("0x{:x}", balance)))
    }

    /// lat_sendRawTransaction - Submit a signed transaction
    pub fn lat_send_raw_transaction(&self, params: Value) -> Result<Value> {
        let params: Vec<Value> = serde_json::from_value(params)
            .map_err(|_| RpcError::invalid_params("Expected array of parameters"))?;

        if params.is_empty() {
            return Err(RpcError::invalid_params("Missing transaction data"));
        }

        let tx_data = params[0]
            .as_str()
            .ok_or_else(|| RpcError::invalid_params("Transaction data must be a hex string"))?;

        // Remove 0x prefix if present
        let tx_hex = tx_data.strip_prefix("0x").unwrap_or(tx_data);

        let tx_bytes =
            hex::decode(tx_hex).map_err(|_| RpcError::invalid_params("Invalid hex encoding"))?;

        let tx: Transaction = borsh::from_slice(&tx_bytes)
            .map_err(|_| RpcError::invalid_transaction("Failed to decode transaction"))?;

        // Validate transaction
        if !tx.verify_signature() {
            return Err(RpcError::invalid_transaction("Invalid signature"));
        }

        let tx_hash = tx.hash();

        // Add to pending transactions
        {
            let mut state = self.state.write();
            state.pending_txs.push(tx.clone());
            state.transactions.insert(tx_hash, (tx, None));
        }

        Ok(json!(format!("0x{}", hex::encode(tx_hash))))
    }

    /// lat_getTransactionReceipt - Get transaction receipt
    pub fn lat_get_transaction_receipt(&self, params: Value) -> Result<Value> {
        let params: Vec<Value> = serde_json::from_value(params)
            .map_err(|_| RpcError::invalid_params("Expected array of parameters"))?;

        if params.is_empty() {
            return Err(RpcError::invalid_params(
                "Missing transaction hash parameter",
            ));
        }

        let hash_str = params[0]
            .as_str()
            .ok_or_else(|| RpcError::invalid_params("Transaction hash must be a string"))?;

        let hash = parse_hash(hash_str)?;

        let state = self.state.read();
        match state.transactions.get(&hash) {
            Some((tx, Some(block_hash))) => {
                // Transaction is included in a block
                let block = state
                    .blocks_by_hash
                    .get(block_hash)
                    .ok_or_else(|| RpcError::internal_error("Block not found"))?;

                let receipt = TransactionReceipt {
                    transaction_hash: format!("0x{}", hex::encode(hash)),
                    block_hash: format!("0x{}", hex::encode(block_hash)),
                    block_number: format!("0x{:x}", block.height()),
                    from: tx.from.to_base58(),
                    to: tx.to.to_base58(),
                    gas_used: format!("0x{:x}", tx.gas_cost()),
                    status: "0x1".to_string(),
                    contract_address: None,
                };
                Ok(json!(receipt))
            }
            Some((_, None)) => {
                // Transaction pending
                Ok(Value::Null)
            }
            None => Ok(Value::Null),
        }
    }

    /// lat_call - Execute a read-only call
    pub fn lat_call(&self, params: Value) -> Result<Value> {
        let params: Vec<Value> = serde_json::from_value(params)
            .map_err(|_| RpcError::invalid_params("Expected array of parameters"))?;

        if params.is_empty() {
            return Err(RpcError::invalid_params("Missing call object"));
        }

        let _call_req: CallRequest = serde_json::from_value(params[0].clone())
            .map_err(|_| RpcError::invalid_params("Invalid call request"))?;

        // Simplified implementation - would integrate with VM for actual execution
        // For now, return empty result
        Ok(json!("0x"))
    }

    /// lat_getWork - Get a mining work template
    ///
    /// Params: [coinbase_address]
    /// Returns: { workId, txCount, header: { version, height, prevHash, txRoot,
    ///            stateRoot, timestamp, difficulty, coinbase } }
    pub fn lat_get_work(&self, params: Value) -> Result<Value> {
        let params: Vec<Value> = serde_json::from_value(params)
            .map_err(|_| RpcError::invalid_params("Expected array of parameters"))?;

        let coinbase_str = params
            .first()
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::invalid_params("Missing coinbase address"))?;

        let coinbase = Address::from_base58(coinbase_str)
            .map_err(|_| RpcError::invalid_params("Invalid coinbase address"))?;

        let mut state = self.state.write();

        let next_height = state.height + 1;

        // Previous block hash: hash of the current best block
        let prev_hash = state
            .blocks_by_height
            .get(&state.height)
            .map(|b| b.hash())
            .unwrap_or([0u8; 32]);

        // Collect pending transactions
        let txs = state.pending_txs.clone();
        let tx_count = txs.len();

        // Merkle root of pending transactions
        let tx_root = Block::calculate_tx_root(&txs);

        // Mainnet policy:
        // - fixed bootstrap difficulty for first 10 blocks
        // - dynamic retarget every 10 blocks afterwards.
        let difficulty = {
            let next_height = state.height + 1;
            let latest = state
                .blocks_by_height
                .get(&state.height)
                .map(|b| b.header.difficulty)
                .unwrap_or(MAINNET_GENESIS_DIFFICULTY);

            if next_height <= MAINNET_DYNAMIC_DIFFICULTY_START_HEIGHT {
                MAINNET_GENESIS_DIFFICULTY
            } else if state.height != 0
                && state
                    .height
                    .is_multiple_of(MAINNET_DYNAMIC_ADJUSTMENT_INTERVAL)
            {
                let interval_start = state
                    .height
                    .saturating_sub(MAINNET_DYNAMIC_ADJUSTMENT_INTERVAL)
                    .saturating_add(1);

                if let (Some(first), Some(last)) = (
                    state.blocks_by_height.get(&interval_start),
                    state.blocks_by_height.get(&state.height),
                ) {
                    let actual_time = last.header.timestamp.saturating_sub(first.header.timestamp);
                    let expected_time =
                        MAINNET_TARGET_BLOCK_TIME_MS * MAINNET_DYNAMIC_ADJUSTMENT_INTERVAL;

                    let ratio = (expected_time as f64 / actual_time.max(1) as f64).clamp(0.25, 4.0);
                    let retarget = (latest as f64 * ratio).round() as u64;
                    retarget.max(1)
                } else {
                    latest
                }
            } else {
                latest
            }
        };

        let timestamp = std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)
            .map(|d| d.as_millis() as u64)
            .unwrap_or(0);

        let header = BlockHeader {
            version: 1,
            height: next_height,
            prev_hash,
            tx_root,
            state_root: [0u8; 32],
            timestamp,
            difficulty,
            nonce: 0,
            coinbase,
        };

        // Use a monotonic counter to generate a unique work_id, ensuring
        // multiple rapid polls for the same height don't produce duplicate entries.
        let seq = self.work_counter.fetch_add(1, Ordering::Relaxed);
        let work_id = format!("{}-{}", next_height, seq);

        // Store the block template keyed by work_id
        let template = Block::new(header.clone(), txs);
        // Evict stale templates for previous heights to bound memory usage
        state
            .pending_work
            .retain(|_, b| b.header.height >= next_height);
        state.pending_work.insert(work_id.clone(), template);

        Ok(json!({
            "workId": work_id,
            "txCount": tx_count,
            "header": {
                "version": header.version,
                "height": format!("0x{:x}", header.height),
                "prevHash": format!("0x{}", hex::encode(header.prev_hash)),
                "txRoot": format!("0x{}", hex::encode(header.tx_root)),
                "stateRoot": format!("0x{}", hex::encode(header.state_root)),
                "timestamp": format!("0x{:x}", header.timestamp),
                "difficulty": format!("0x{:x}", header.difficulty),
                "coinbase": header.coinbase.to_base58(),
            }
        }))
    }

    /// lat_submitWork - Submit a found mining solution
    ///
    /// Params: [{ workId, nonce, powHash }]
    /// Returns: true if the solution was accepted, false otherwise
    pub fn lat_submit_work(&self, params: Value) -> Result<Value> {
        let params: Vec<Value> = serde_json::from_value(params)
            .map_err(|_| RpcError::invalid_params("Expected array of parameters"))?;

        let submission = params
            .first()
            .ok_or_else(|| RpcError::invalid_params("Missing submission object"))?;

        let work_id = submission
            .get("workId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::invalid_params("Missing workId"))?
            .to_string();

        let nonce_str = submission
            .get("nonce")
            .and_then(|v| v.as_str())
            .ok_or_else(|| RpcError::invalid_params("Missing nonce"))?;

        let nonce = parse_hex_u64(nonce_str)
            .map_err(|_| RpcError::invalid_params("Invalid nonce format"))?;

        let mut state = self.state.write();

        // Retrieve (and remove) the work template
        let mut block = match state.pending_work.remove(&work_id) {
            Some(b) => b,
            None => {
                debug!(work_id = %work_id, "submitWork: unknown or stale work_id");
                return Ok(json!(false));
            }
        };

        // Insert the found nonce and verify the PoW
        block.header.nonce = nonce;
        let valid = match verify_pow(&block.header, &state.pow_config) {
            Ok(v) => v,
            Err(e) => {
                warn!(work_id = %work_id, error = %e, "submitWork: PoW verification error");
                return Ok(json!(false));
            }
        };

        if !valid {
            debug!(work_id = %work_id, nonce, "submitWork: nonce does not meet difficulty target");
            return Ok(json!(false));
        }

        let block_hash = block.hash();
        let block_height = block.header.height;

        // Reject if a block was already accepted at this height (no reorganisation support)
        if state.blocks_by_height.contains_key(&block_height) {
            debug!(
                height = block_height,
                "submitWork: block already accepted at this height"
            );
            return Ok(json!(false));
        }

        // Move block transactions from pending to confirmed
        let block_tx_hashes: std::collections::HashSet<Hash> =
            block.transactions.iter().map(|tx| tx.hash()).collect();
        state
            .pending_txs
            .retain(|tx| !block_tx_hashes.contains(&tx.hash()));

        for tx in &block.transactions {
            let tx_hash = tx.hash();
            state
                .transactions
                .entry(tx_hash)
                .and_modify(|(_, bh)| *bh = Some(block_hash))
                .or_insert_with(|| (tx.clone(), Some(block_hash)));
        }

        // Add the new block to the chain
        state.blocks_by_height.insert(block_height, block.clone());
        state.blocks_by_hash.insert(block_hash, block);

        if block_height > state.height {
            state.height = block_height;
        }

        debug!(height = block_height, "submitWork: block accepted");
        Ok(json!(true))
    }

    /// lat_estimateGas - Estimate gas for a transaction
    pub fn lat_estimate_gas(&self, params: Value) -> Result<Value> {
        let params: Vec<Value> = serde_json::from_value(params)
            .map_err(|_| RpcError::invalid_params("Expected array of parameters"))?;

        if params.is_empty() {
            return Err(RpcError::invalid_params("Missing transaction object"));
        }

        let call_req: CallRequest = serde_json::from_value(params[0].clone())
            .map_err(|_| RpcError::invalid_params("Invalid call request"))?;

        // Calculate base gas + data gas
        let base_gas: u64 = 21000;
        let data_gas: u64 = call_req
            .data
            .as_ref()
            .map(|d| {
                let hex = d.strip_prefix("0x").unwrap_or(d);
                (hex.len() / 2) as u64 * 16
            })
            .unwrap_or(0);

        let estimated_gas = base_gas + data_gas;
        Ok(json!(format!("0x{:x}", estimated_gas)))
    }

    /// Convert block to RPC format
    fn block_to_rpc(&self, block: &Block, include_txs: bool) -> RpcBlock {
        let txs: Vec<Value> = if include_txs {
            block
                .transactions
                .iter()
                .map(|tx| {
                    let rpc_tx = self.transaction_to_rpc(tx, Some(&block.hash()));
                    serde_json::to_value(rpc_tx).unwrap_or(Value::Null)
                })
                .collect()
        } else {
            block
                .transactions
                .iter()
                .map(|tx| json!(format!("0x{}", hex::encode(tx.hash()))))
                .collect()
        };

        RpcBlock {
            number: format!("0x{:x}", block.header.height),
            hash: format!("0x{}", hex::encode(block.hash())),
            parent_hash: format!("0x{}", hex::encode(block.header.prev_hash)),
            timestamp: format!("0x{:x}", block.header.timestamp),
            difficulty: format!("0x{:x}", block.header.difficulty),
            nonce: format!("0x{:x}", block.header.nonce),
            miner: block.header.coinbase.to_base58(),
            transactions_root: format!("0x{}", hex::encode(block.header.tx_root)),
            state_root: format!("0x{}", hex::encode(block.header.state_root)),
            transactions: txs,
        }
    }

    /// Convert transaction to RPC format
    fn transaction_to_rpc(&self, tx: &Transaction, block_hash: Option<&Hash>) -> RpcTransaction {
        let state = self.state.read();

        let (block_hash_str, block_number_str) = match block_hash {
            Some(hash) => {
                let block = state.blocks_by_hash.get(hash);
                (
                    Some(format!("0x{}", hex::encode(hash))),
                    block.map(|b| format!("0x{:x}", b.height())),
                )
            }
            None => (None, None),
        };

        RpcTransaction {
            hash: format!("0x{}", hex::encode(tx.hash())),
            from: tx.from.to_base58(),
            to: tx.to.to_base58(),
            value: format!("0x{:x}", tx.amount),
            gas: format!("0x{:x}", tx.gas_limit),
            nonce: format!("0x{:x}", tx.nonce),
            input: format!("0x{}", hex::encode(&tx.data)),
            block_hash: block_hash_str,
            block_number: block_number_str,
        }
    }
}

/// Parse a hex string (with or without 0x prefix) into a u64
fn parse_hex_u64(s: &str) -> std::result::Result<u64, std::num::ParseIntError> {
    let hex_str = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(hex_str, 16)
}

/// Parse a hex-encoded hash string
fn parse_hash(s: &str) -> Result<Hash> {
    let hex_str = s.strip_prefix("0x").unwrap_or(s);
    let bytes = hex::decode(hex_str).map_err(|_| RpcError::invalid_params("Invalid hex format"))?;

    if bytes.len() != 32 {
        return Err(RpcError::invalid_params("Hash must be 32 bytes"));
    }

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_block_number() {
        let handlers = RpcHandlers::new();
        let result = handlers.lat_block_number().unwrap();
        assert_eq!(result, json!("0x0"));
    }

    #[test]
    fn test_get_genesis_block() {
        let handlers = RpcHandlers::new();
        let params = json!([0, false]);
        let result = handlers.lat_get_block_by_number(params).unwrap();
        assert!(result.get("number").is_some());
    }

    #[test]
    fn test_estimate_gas() {
        let handlers = RpcHandlers::new();
        let params = json!([{
            "to": "1111111111111111111111111111111",
            "data": "0x1234"
        }]);
        let result = handlers.lat_estimate_gas(params).unwrap();
        // base (21000) + 2 bytes of data (32) = 21032 = 0x5228
        assert_eq!(result, json!("0x5228"));
    }

    #[test]
    fn test_unknown_method() {
        let handlers = RpcHandlers::new();
        let result = handlers.handle("lat_unknown", json!([]));
        assert!(result.is_err());
    }
}
