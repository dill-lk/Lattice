//! Host functions exposed to WASM contracts
//!
//! These functions provide the interface between the WASM runtime
//! and the blockchain state.

use crate::error::{Result, VmError};
use crate::gas::GasMeter;
use lattice_core::{Address, Amount, BlockHeight, Hash, Timestamp};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};

pub type ContractStorage = HashMap<Address, HashMap<Vec<u8>, Vec<u8>>>;
pub type SharedContractStorage = Arc<Mutex<ContractStorage>>;

/// Maximum storage key length
pub const MAX_STORAGE_KEY_LEN: usize = 256;
/// Maximum storage value length
pub const MAX_STORAGE_VALUE_LEN: usize = 65536;
/// Maximum log data length
pub const MAX_LOG_DATA_LEN: usize = 16384;
/// Maximum number of log topics
pub const MAX_LOG_TOPICS: usize = 4;

/// Event log emitted by a contract
#[derive(Debug, Clone)]
pub struct Log {
    /// Address of the contract that emitted the log
    pub address: Address,
    /// Indexed topics for filtering
    pub topics: Vec<Hash>,
    /// Unindexed data
    pub data: Vec<u8>,
}

/// Block context for contract execution
#[derive(Debug, Clone)]
pub struct BlockContext {
    /// Current block height
    pub height: BlockHeight,
    /// Current block timestamp
    pub timestamp: Timestamp,
    /// Current block difficulty
    pub difficulty: u64,
    /// Gas limit for the block
    pub gas_limit: u64,
    /// Coinbase address (miner)
    pub coinbase: Address,
    /// Previous block hash
    pub prev_hash: Hash,
}

impl Default for BlockContext {
    fn default() -> Self {
        Self {
            height: 0,
            timestamp: 0,
            difficulty: 1,
            gas_limit: 10_000_000,
            coinbase: Address::zero(),
            prev_hash: [0u8; 32],
        }
    }
}

/// Call context for the current execution frame
#[derive(Debug, Clone)]
pub struct CallContext {
    /// Address of the caller
    pub caller: Address,
    /// Address of the contract being executed
    pub address: Address,
    /// Value transferred with the call
    pub value: Amount,
    /// Input data for the call
    pub input: Vec<u8>,
    /// Current call depth
    pub depth: u32,
}

/// Host functions interface for WASM contracts
#[derive(Debug, Clone)]
pub struct HostFunctions {
    /// Gas meter for this execution
    gas_meter: Arc<Mutex<GasMeter>>,
    /// Contract storage (address -> key -> value)
    storage: SharedContractStorage,
    /// Contract code storage (code_hash -> code)
    code: Arc<Mutex<HashMap<Hash, Vec<u8>>>>,
    /// Account balances
    balances: Arc<Mutex<HashMap<Address, Amount>>>,
    /// Emitted logs
    logs: Arc<Mutex<Vec<Log>>>,
    /// Block context
    block: BlockContext,
    /// Call context
    call: CallContext,
}

impl HostFunctions {
    /// Create new host functions with the given contexts
    pub fn new(gas_meter: GasMeter, block: BlockContext, call: CallContext) -> Self {
        Self {
            gas_meter: Arc::new(Mutex::new(gas_meter)),
            storage: Arc::new(Mutex::new(HashMap::new())),
            code: Arc::new(Mutex::new(HashMap::new())),
            balances: Arc::new(Mutex::new(HashMap::new())),
            logs: Arc::new(Mutex::new(Vec::new())),
            block,
            call,
        }
    }

    /// Create with shared state (for nested calls)
    pub fn with_shared_state(
        gas_meter: Arc<Mutex<GasMeter>>,
        storage: SharedContractStorage,
        code: Arc<Mutex<HashMap<Hash, Vec<u8>>>>,
        balances: Arc<Mutex<HashMap<Address, Amount>>>,
        logs: Arc<Mutex<Vec<Log>>>,
        block: BlockContext,
        call: CallContext,
    ) -> Self {
        Self {
            gas_meter,
            storage,
            code,
            balances,
            logs,
            block,
            call,
        }
    }

    /// Get the gas meter
    pub fn gas_meter(&self) -> &Arc<Mutex<GasMeter>> {
        &self.gas_meter
    }

    /// Get collected logs
    pub fn logs(&self) -> Vec<Log> {
        self.logs.lock().unwrap().clone()
    }

    /// Get the call context
    pub fn call_context(&self) -> &CallContext {
        &self.call
    }

    /// Get the block context
    pub fn block_context(&self) -> &BlockContext {
        &self.block
    }

    // ========== Storage Operations ==========

    /// Read from contract storage
    pub fn storage_read(&self, key: &[u8]) -> Result<Option<Vec<u8>>> {
        if key.len() > MAX_STORAGE_KEY_LEN {
            return Err(VmError::StorageKeyTooLong(key.len()));
        }

        {
            let mut meter = self.gas_meter.lock().unwrap();
            meter.charge_storage_read()?;
        }

        let storage = self.storage.lock().unwrap();
        Ok(storage
            .get(&self.call.address)
            .and_then(|s| s.get(key))
            .cloned())
    }

    /// Write to contract storage
    pub fn storage_write(&self, key: &[u8], value: &[u8]) -> Result<()> {
        if key.len() > MAX_STORAGE_KEY_LEN {
            return Err(VmError::StorageKeyTooLong(key.len()));
        }
        if value.len() > MAX_STORAGE_VALUE_LEN {
            return Err(VmError::StorageValueTooLong(value.len()));
        }

        let is_new = {
            let storage = self.storage.lock().unwrap();
            !storage
                .get(&self.call.address)
                .map(|s| s.contains_key(key))
                .unwrap_or(false)
        };

        {
            let mut meter = self.gas_meter.lock().unwrap();
            meter.charge_storage_write(is_new, value.len())?;
        }

        let mut storage = self.storage.lock().unwrap();
        storage
            .entry(self.call.address.clone())
            .or_default()
            .insert(key.to_vec(), value.to_vec());

        Ok(())
    }

    /// Delete from contract storage
    pub fn storage_delete(&self, key: &[u8]) -> Result<()> {
        if key.len() > MAX_STORAGE_KEY_LEN {
            return Err(VmError::StorageKeyTooLong(key.len()));
        }

        {
            let mut meter = self.gas_meter.lock().unwrap();
            meter.charge_storage_delete()?;
        }

        let mut storage = self.storage.lock().unwrap();
        if let Some(contract_storage) = storage.get_mut(&self.call.address) {
            contract_storage.remove(key);
        }

        Ok(())
    }

    // ========== Context Accessors ==========

    /// Get the caller address
    pub fn get_caller(&self) -> Result<Address> {
        {
            let mut meter = self.gas_meter.lock().unwrap();
            let caller_cost = meter.costs().caller;
            meter.charge(caller_cost)?;
        }
        Ok(self.call.caller.clone())
    }

    /// Get the current contract address
    pub fn get_address(&self) -> Result<Address> {
        {
            let mut meter = self.gas_meter.lock().unwrap();
            let address_cost = meter.costs().address;
            meter.charge(address_cost)?;
        }
        Ok(self.call.address.clone())
    }

    /// Get the value sent with the call
    pub fn get_value(&self) -> Amount {
        self.call.value
    }

    /// Get the input data
    pub fn get_input(&self) -> &[u8] {
        &self.call.input
    }

    /// Get the current call depth
    pub fn get_depth(&self) -> u32 {
        self.call.depth
    }

    // ========== Block Information ==========

    /// Get the current block height
    pub fn get_block_height(&self) -> Result<BlockHeight> {
        {
            let mut meter = self.gas_meter.lock().unwrap();
            let block_info_cost = meter.costs().block_info;
            meter.charge(block_info_cost)?;
        }
        Ok(self.block.height)
    }

    /// Get the current block timestamp
    pub fn get_block_timestamp(&self) -> Result<Timestamp> {
        {
            let mut meter = self.gas_meter.lock().unwrap();
            let block_info_cost = meter.costs().block_info;
            meter.charge(block_info_cost)?;
        }
        Ok(self.block.timestamp)
    }

    /// Get the current block difficulty
    pub fn get_block_difficulty(&self) -> Result<u64> {
        {
            let mut meter = self.gas_meter.lock().unwrap();
            let block_info_cost = meter.costs().block_info;
            meter.charge(block_info_cost)?;
        }
        Ok(self.block.difficulty)
    }

    /// Get the block gas limit
    pub fn get_block_gas_limit(&self) -> Result<u64> {
        {
            let mut meter = self.gas_meter.lock().unwrap();
            let block_info_cost = meter.costs().block_info;
            meter.charge(block_info_cost)?;
        }
        Ok(self.block.gas_limit)
    }

    /// Get the coinbase address
    pub fn get_coinbase(&self) -> Result<Address> {
        {
            let mut meter = self.gas_meter.lock().unwrap();
            let block_info_cost = meter.costs().block_info;
            meter.charge(block_info_cost)?;
        }
        Ok(self.block.coinbase.clone())
    }

    /// Get the previous block hash
    pub fn get_prev_hash(&self) -> Result<Hash> {
        {
            let mut meter = self.gas_meter.lock().unwrap();
            let block_info_cost = meter.costs().block_info;
            meter.charge(block_info_cost)?;
        }
        Ok(self.block.prev_hash)
    }

    // ========== Balance Operations ==========

    /// Get balance of an address
    pub fn get_balance(&self, address: &Address) -> Result<Amount> {
        {
            let mut meter = self.gas_meter.lock().unwrap();
            let balance_cost = meter.costs().balance;
            meter.charge(balance_cost)?;
        }
        let balances = self.balances.lock().unwrap();
        Ok(*balances.get(address).unwrap_or(&0))
    }

    /// Set balance of an address (internal use)
    pub fn set_balance(&self, address: &Address, amount: Amount) {
        let mut balances = self.balances.lock().unwrap();
        balances.insert(address.clone(), amount);
    }

    /// Transfer value between addresses
    pub fn transfer(&self, to: &Address, amount: Amount) -> Result<()> {
        let from = &self.call.address;
        let mut balances = self.balances.lock().unwrap();

        let from_balance = *balances.get(from).unwrap_or(&0);
        if from_balance < amount {
            return Err(VmError::InsufficientBalance);
        }

        balances.insert(from.clone(), from_balance - amount);
        let to_balance = *balances.get(to).unwrap_or(&0);
        balances.insert(to.clone(), to_balance + amount);

        Ok(())
    }

    // ========== Logging ==========

    /// Emit a log event
    pub fn emit_log(&self, topics: Vec<Hash>, data: Vec<u8>) -> Result<()> {
        if topics.len() > MAX_LOG_TOPICS {
            return Err(VmError::HostError(format!(
                "too many log topics: {} (max {})",
                topics.len(),
                MAX_LOG_TOPICS
            )));
        }
        if data.len() > MAX_LOG_DATA_LEN {
            return Err(VmError::HostError(format!(
                "log data too long: {} (max {})",
                data.len(),
                MAX_LOG_DATA_LEN
            )));
        }

        {
            let mut meter = self.gas_meter.lock().unwrap();
            meter.charge_log(data.len(), topics.len())?;
        }

        let log = Log {
            address: self.call.address.clone(),
            topics,
            data,
        };

        self.logs.lock().unwrap().push(log);
        Ok(())
    }

    // ========== Crypto Helpers ==========

    /// Compute SHA3-256 hash
    pub fn sha3(&self, data: &[u8]) -> Result<Hash> {
        use sha3::{Digest, Sha3_256};

        {
            let mut meter = self.gas_meter.lock().unwrap();
            meter.charge_sha3(data.len())?;
        }

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&Sha3_256::digest(data));
        Ok(hash)
    }

    /// Verify a Dilithium signature
    pub fn verify_signature(
        &self,
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool> {
        {
            let mut meter = self.gas_meter.lock().unwrap();
            meter.charge_signature_verify()?;
        }

        // Use lattice-crypto for verification
        let pk = match lattice_crypto::PublicKey::from_bytes(public_key) {
            Ok(pk) => pk,
            Err(_) => return Ok(false),
        };

        let sig = lattice_crypto::Signature::from_bytes(signature);

        Ok(lattice_crypto::verify(message, &sig, &pk).is_ok())
    }

    // ========== Code Management ==========

    /// Get contract code by hash
    pub fn get_code(&self, code_hash: &Hash) -> Option<Vec<u8>> {
        self.code.lock().unwrap().get(code_hash).cloned()
    }

    /// Store contract code
    pub fn store_code(&self, code: Vec<u8>) -> Hash {
        use sha3::{Digest, Sha3_256};

        let mut hash = [0u8; 32];
        hash.copy_from_slice(&Sha3_256::digest(&code));

        self.code.lock().unwrap().insert(hash, code);
        hash
    }

    // ========== State Accessors for Testing ==========

    /// Get all storage for an address (for testing/debugging)
    pub fn get_all_storage(&self, address: &Address) -> HashMap<Vec<u8>, Vec<u8>> {
        self.storage
            .lock()
            .unwrap()
            .get(address)
            .cloned()
            .unwrap_or_default()
    }

    /// Get shared storage reference
    pub fn storage_ref(&self) -> SharedContractStorage {
        Arc::clone(&self.storage)
    }

    /// Get shared code reference
    pub fn code_ref(&self) -> Arc<Mutex<HashMap<Hash, Vec<u8>>>> {
        Arc::clone(&self.code)
    }

    /// Get shared balances reference
    pub fn balances_ref(&self) -> Arc<Mutex<HashMap<Address, Amount>>> {
        Arc::clone(&self.balances)
    }

    /// Get shared logs reference
    pub fn logs_ref(&self) -> Arc<Mutex<Vec<Log>>> {
        Arc::clone(&self.logs)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn test_host() -> HostFunctions {
        let gas_meter = GasMeter::new(1_000_000);
        let block = BlockContext::default();
        let call = CallContext {
            caller: Address::from_bytes([1u8; 20]),
            address: Address::from_bytes([2u8; 20]),
            value: 0,
            input: vec![],
            depth: 0,
        };
        HostFunctions::new(gas_meter, block, call)
    }

    #[test]
    fn test_storage_roundtrip() {
        let host = test_host();

        host.storage_write(b"key1", b"value1").unwrap();
        let value = host.storage_read(b"key1").unwrap();
        assert_eq!(value, Some(b"value1".to_vec()));
    }

    #[test]
    fn test_storage_delete() {
        let host = test_host();

        host.storage_write(b"key1", b"value1").unwrap();
        host.storage_delete(b"key1").unwrap();
        let value = host.storage_read(b"key1").unwrap();
        assert_eq!(value, None);
    }

    #[test]
    fn test_storage_key_too_long() {
        let host = test_host();
        let long_key = vec![0u8; MAX_STORAGE_KEY_LEN + 1];

        let result = host.storage_read(&long_key);
        assert!(matches!(result, Err(VmError::StorageKeyTooLong(_))));
    }

    #[test]
    fn test_emit_log() {
        let host = test_host();

        let topic = [1u8; 32];
        host.emit_log(vec![topic], b"test data".to_vec()).unwrap();

        let logs = host.logs();
        assert_eq!(logs.len(), 1);
        assert_eq!(logs[0].data, b"test data");
        assert_eq!(logs[0].topics, vec![topic]);
    }

    #[test]
    fn test_sha3() {
        let host = test_host();
        let hash = host.sha3(b"hello").unwrap();
        assert_eq!(hash.len(), 32);
    }

    #[test]
    fn test_get_caller() {
        let host = test_host();
        let caller = host.get_caller().unwrap();
        assert_eq!(caller, Address::from_bytes([1u8; 20]));
    }

    #[test]
    fn test_balance_transfer() {
        let host = test_host();

        // Set initial balance
        host.set_balance(&host.call.address, 1000);

        let to = Address::from_bytes([3u8; 20]);
        host.transfer(&to, 400).unwrap();

        assert_eq!(host.get_balance(&host.call.address).unwrap(), 600);
        assert_eq!(host.get_balance(&to).unwrap(), 400);
    }

    #[test]
    fn test_insufficient_balance() {
        let host = test_host();

        host.set_balance(&host.call.address, 100);
        let to = Address::from_bytes([3u8; 20]);

        let result = host.transfer(&to, 500);
        assert!(matches!(result, Err(VmError::InsufficientBalance)));
    }
}
