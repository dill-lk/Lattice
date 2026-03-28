//! WASM runtime for smart contract execution
//!
//! Provides sandboxed execution of WebAssembly contracts with gas metering.

use crate::error::{Result, VmError};
use crate::gas::GasMeter;
use crate::host::{BlockContext, CallContext, HostFunctions, Log};
use lattice_core::{Address, Amount, Hash};
use sha3::{Digest, Sha3_256};
use std::collections::HashMap;
use std::sync::{Arc, Mutex};
use tracing::{debug, trace};
use wasmer::{
    imports, Function, FunctionEnv, FunctionEnvMut, Instance, Memory, MemoryType, Module, Pages,
    Store, TypedFunction, Value,
};

/// Maximum call depth for nested contract calls
const MAX_CALL_DEPTH: u32 = 64;

/// Maximum contract code size (128 KB)
const MAX_CODE_SIZE: usize = 128 * 1024;

/// Maximum memory pages (1 page = 64KB)
const MAX_MEMORY_PAGES: u32 = 256;

/// Execution result from contract call
#[derive(Debug, Clone)]
pub struct ExecutionResult {
    /// Return data from the contract
    pub return_data: Vec<u8>,
    /// Gas used during execution
    pub gas_used: u64,
    /// Logs emitted during execution
    pub logs: Vec<Log>,
    /// Whether execution was successful
    pub success: bool,
    /// Error message if execution failed
    pub error: Option<String>,
}

/// Contract deployment result
#[derive(Debug, Clone)]
pub struct DeploymentResult {
    /// Address of the deployed contract
    pub address: Address,
    /// Hash of the contract code
    pub code_hash: Hash,
    /// Gas used for deployment
    pub gas_used: u64,
}

/// Environment passed to host functions
struct WasmEnv {
    host: HostFunctions,
    memory: Option<Memory>,
    return_data: Vec<u8>,
}

/// WASM runtime for executing smart contracts
pub struct Runtime {
    /// Wasmer store
    store: Store,
    /// Contract code cache (code_hash -> compiled module)
    module_cache: HashMap<Hash, Module>,
    /// Contract storage
    storage: Arc<Mutex<HashMap<Address, HashMap<Vec<u8>, Vec<u8>>>>>,
    /// Contract code storage
    code: Arc<Mutex<HashMap<Hash, Vec<u8>>>>,
    /// Account balances
    balances: Arc<Mutex<HashMap<Address, Amount>>>,
    /// Address to code hash mapping
    address_code: HashMap<Address, Hash>,
}

impl Runtime {
    /// Create a new runtime
    pub fn new() -> Self {
        Self {
            store: Store::default(),
            module_cache: HashMap::new(),
            storage: Arc::new(Mutex::new(HashMap::new())),
            code: Arc::new(Mutex::new(HashMap::new())),
            balances: Arc::new(Mutex::new(HashMap::new())),
            address_code: HashMap::new(),
        }
    }

    /// Deploy a new contract
    pub fn deploy(
        &mut self,
        code: Vec<u8>,
        deployer: Address,
        value: Amount,
        gas_limit: u64,
        block: BlockContext,
        init_data: Vec<u8>,
    ) -> Result<DeploymentResult> {
        // Validate code size
        if code.len() > MAX_CODE_SIZE {
            return Err(VmError::InvalidModule(format!(
                "code size {} exceeds max {}",
                code.len(),
                MAX_CODE_SIZE
            )));
        }

        // Compute code hash
        let code_hash = {
            let mut hash = [0u8; 32];
            hash.copy_from_slice(&Sha3_256::digest(&code));
            hash
        };

        // Generate contract address from deployer + nonce (simplified)
        let contract_address = {
            let mut hasher = Sha3_256::new();
            hasher.update(deployer.as_bytes());
            hasher.update(&code_hash);
            let digest = hasher.finalize();
            let mut addr = [0u8; 20];
            addr.copy_from_slice(&digest[..20]);
            Address::from_bytes(addr)
        };

        debug!(
            code_hash = hex::encode(code_hash),
            address = %contract_address,
            "Deploying contract"
        );

        // Compile the module
        let module = Module::new(&self.store, &code).map_err(|e| {
            VmError::CompilationError(format!("failed to compile WASM: {}", e))
        })?;

        // Create gas meter and charge for deployment
        let mut gas_meter = GasMeter::new(gas_limit);
        gas_meter.charge_create(code.len())?;

        // Store the code
        self.code.lock().unwrap().insert(code_hash, code);
        self.module_cache.insert(code_hash, module.clone());
        self.address_code.insert(contract_address.clone(), code_hash);

        // Set initial balance
        {
            let mut balances = self.balances.lock().unwrap();
            *balances.entry(contract_address.clone()).or_default() += value;
        }

        // Run constructor if init_data is provided
        if !init_data.is_empty() {
            let call_context = CallContext {
                caller: deployer,
                address: contract_address.clone(),
                value,
                input: init_data,
                depth: 0,
            };

            let host = HostFunctions::with_shared_state(
                Arc::new(Mutex::new(gas_meter.clone())),
                Arc::clone(&self.storage),
                Arc::clone(&self.code),
                Arc::clone(&self.balances),
                Arc::new(Mutex::new(Vec::new())),
                block,
                call_context,
            );

            self.execute_internal(host, &module, "init")?;
        }

        Ok(DeploymentResult {
            address: contract_address,
            code_hash,
            gas_used: gas_meter.used(),
        })
    }

    /// Execute a contract call
    pub fn call(
        &mut self,
        contract: Address,
        caller: Address,
        value: Amount,
        input: Vec<u8>,
        gas_limit: u64,
        block: BlockContext,
    ) -> Result<ExecutionResult> {
        // Get contract code hash
        let code_hash = self
            .address_code
            .get(&contract)
            .ok_or_else(|| VmError::ContractNotFound(contract.to_string()))?
            .clone();

        // Get or compile module
        let module = self.get_or_compile_module(&code_hash)?;

        // Create execution context
        let gas_meter = GasMeter::new(gas_limit);
        let logs = Arc::new(Mutex::new(Vec::new()));

        let call_context = CallContext {
            caller,
            address: contract,
            value,
            input,
            depth: 0,
        };

        let host = HostFunctions::with_shared_state(
            Arc::new(Mutex::new(gas_meter)),
            Arc::clone(&self.storage),
            Arc::clone(&self.code),
            Arc::clone(&self.balances),
            Arc::clone(&logs),
            block,
            call_context,
        );

        // Execute the call
        match self.execute_internal(host.clone(), &module, "call") {
            Ok(return_data) => {
                let gas_used = host.gas_meter().lock().unwrap().used();
                Ok(ExecutionResult {
                    return_data,
                    gas_used,
                    logs: logs.lock().unwrap().clone(),
                    success: true,
                    error: None,
                })
            }
            Err(e) => {
                let gas_used = host.gas_meter().lock().unwrap().used();
                Ok(ExecutionResult {
                    return_data: vec![],
                    gas_used,
                    logs: vec![],
                    success: false,
                    error: Some(e.to_string()),
                })
            }
        }
    }

    /// Execute a static call (read-only, no state changes)
    pub fn static_call(
        &mut self,
        contract: Address,
        caller: Address,
        input: Vec<u8>,
        gas_limit: u64,
        block: BlockContext,
    ) -> Result<ExecutionResult> {
        // Use call with zero value and snapshot storage
        let storage_snapshot = self.storage.lock().unwrap().clone();

        let result = self.call(contract, caller, 0, input, gas_limit, block);

        // Restore storage snapshot for static call
        *self.storage.lock().unwrap() = storage_snapshot;

        result
    }

    /// Get or compile a module from code hash
    fn get_or_compile_module(&mut self, code_hash: &Hash) -> Result<Module> {
        if let Some(module) = self.module_cache.get(code_hash) {
            return Ok(module.clone());
        }

        let code = self
            .code
            .lock()
            .unwrap()
            .get(code_hash)
            .cloned()
            .ok_or_else(|| VmError::ContractNotFound(hex::encode(code_hash)))?;

        let module = Module::new(&self.store, &code)
            .map_err(|e| VmError::CompilationError(e.to_string()))?;

        self.module_cache.insert(*code_hash, module.clone());
        Ok(module)
    }

    /// Internal execution logic
    fn execute_internal(
        &mut self,
        host: HostFunctions,
        module: &Module,
        entry_point: &str,
    ) -> Result<Vec<u8>> {
        // Check call depth
        if host.call_context().depth > MAX_CALL_DEPTH {
            return Err(VmError::CallDepthExceeded(host.call_context().depth));
        }

        let mut store = Store::default();

        // Create environment
        let env = WasmEnv {
            host,
            memory: None,
            return_data: vec![],
        };
        let env = FunctionEnv::new(&mut store, env);

        // Create memory
        let memory_type = MemoryType::new(1, Some(MAX_MEMORY_PAGES), false);
        let memory = Memory::new(&mut store, memory_type)
            .map_err(|e| VmError::InstantiationError(e.to_string()))?;

        // Store memory reference
        env.as_mut(&mut store).memory = Some(memory.clone());

        // Create imports
        let import_object = imports! {
            "env" => {
                "memory" => memory,
                // Storage functions
                "storage_read" => Function::new_typed_with_env(&mut store, &env, host_storage_read),
                "storage_write" => Function::new_typed_with_env(&mut store, &env, host_storage_write),
                "storage_delete" => Function::new_typed_with_env(&mut store, &env, host_storage_delete),
                // Context functions
                "get_caller" => Function::new_typed_with_env(&mut store, &env, host_get_caller),
                "get_address" => Function::new_typed_with_env(&mut store, &env, host_get_address),
                "get_value" => Function::new_typed_with_env(&mut store, &env, host_get_value),
                "get_input_len" => Function::new_typed_with_env(&mut store, &env, host_get_input_len),
                "get_input" => Function::new_typed_with_env(&mut store, &env, host_get_input),
                // Block functions
                "get_block_height" => Function::new_typed_with_env(&mut store, &env, host_get_block_height),
                "get_block_timestamp" => Function::new_typed_with_env(&mut store, &env, host_get_block_timestamp),
                // Crypto functions
                "sha3" => Function::new_typed_with_env(&mut store, &env, host_sha3),
                "verify_signature" => Function::new_typed_with_env(&mut store, &env, host_verify_signature),
                // Output functions
                "set_return" => Function::new_typed_with_env(&mut store, &env, host_set_return),
                "emit_log" => Function::new_typed_with_env(&mut store, &env, host_emit_log),
                // Abort
                "abort" => Function::new_typed_with_env(&mut store, &env, host_abort),
            }
        };

        // Instantiate module
        let instance = Instance::new(&mut store, module, &import_object)
            .map_err(|e| VmError::InstantiationError(e.to_string()))?;

        // Get and call entry point
        let entry: TypedFunction<(), ()> = instance
            .exports
            .get_typed_function(&store, entry_point)
            .map_err(|_| VmError::EntryPointNotFound(entry_point.to_string()))?;

        trace!(entry_point, "Calling contract entry point");

        entry.call(&mut store).map_err(|e| {
            let msg = e.to_string();
            if msg.contains("out of gas") {
                VmError::OutOfGas {
                    required: 0,
                    available: 0,
                }
            } else if msg.contains("unreachable") {
                VmError::Trap(msg)
            } else {
                VmError::ExecutionError(msg)
            }
        })?;

        // Get return data
        let return_data = env.as_ref(&store).return_data.clone();
        Ok(return_data)
    }

    /// Set balance for an address
    pub fn set_balance(&mut self, address: &Address, amount: Amount) {
        self.balances.lock().unwrap().insert(address.clone(), amount);
    }

    /// Get balance for an address
    pub fn get_balance(&self, address: &Address) -> Amount {
        *self.balances.lock().unwrap().get(address).unwrap_or(&0)
    }

    /// Get storage value
    pub fn get_storage(&self, address: &Address, key: &[u8]) -> Option<Vec<u8>> {
        self.storage
            .lock()
            .unwrap()
            .get(address)
            .and_then(|s| s.get(key).cloned())
    }

    /// Check if contract exists
    pub fn has_contract(&self, address: &Address) -> bool {
        self.address_code.contains_key(address)
    }
}

impl Default for Runtime {
    fn default() -> Self {
        Self::new()
    }
}

// ========== Host Function Implementations ==========

fn read_memory(env: &WasmEnv, store: &impl wasmer::AsStoreRef, offset: u32, len: u32) -> Result<Vec<u8>> {
    let memory = env
        .memory
        .as_ref()
        .ok_or_else(|| VmError::HostError("memory not initialized".into()))?;

    let view = memory.view(store);
    let mut buffer = vec![0u8; len as usize];
    view.read(offset as u64, &mut buffer)
        .map_err(|_| VmError::MemoryOutOfBounds { offset, length: len })?;
    Ok(buffer)
}

fn write_memory(
    env: &WasmEnv,
    store: &mut impl wasmer::AsStoreMut,
    offset: u32,
    data: &[u8],
) -> Result<()> {
    let memory = env
        .memory
        .as_ref()
        .ok_or_else(|| VmError::HostError("memory not initialized".into()))?;

    let view = memory.view(store);
    view.write(offset as u64, data)
        .map_err(|_| VmError::MemoryOutOfBounds {
            offset,
            length: data.len() as u32,
        })?;
    Ok(())
}

fn host_storage_read(mut env: FunctionEnvMut<WasmEnv>, key_ptr: u32, key_len: u32, value_ptr: u32) -> i32 {
    let (data, store) = env.data_and_store_mut();

    let key = match read_memory(data, &store, key_ptr, key_len) {
        Ok(k) => k,
        Err(_) => return -1,
    };

    match data.host.storage_read(&key) {
        Ok(Some(value)) => {
            if write_memory(data, &mut env.as_store_mut(), value_ptr, &value).is_err() {
                return -1;
            }
            value.len() as i32
        }
        Ok(None) => 0,
        Err(_) => -1,
    }
}

fn host_storage_write(
    mut env: FunctionEnvMut<WasmEnv>,
    key_ptr: u32,
    key_len: u32,
    value_ptr: u32,
    value_len: u32,
) -> i32 {
    let (data, store) = env.data_and_store_mut();

    let key = match read_memory(data, &store, key_ptr, key_len) {
        Ok(k) => k,
        Err(_) => return -1,
    };

    let value = match read_memory(data, &store, value_ptr, value_len) {
        Ok(v) => v,
        Err(_) => return -1,
    };

    match data.host.storage_write(&key, &value) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

fn host_storage_delete(mut env: FunctionEnvMut<WasmEnv>, key_ptr: u32, key_len: u32) -> i32 {
    let (data, store) = env.data_and_store_mut();

    let key = match read_memory(data, &store, key_ptr, key_len) {
        Ok(k) => k,
        Err(_) => return -1,
    };

    match data.host.storage_delete(&key) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

fn host_get_caller(mut env: FunctionEnvMut<WasmEnv>, ptr: u32) -> i32 {
    let data = env.data_mut();
    let caller = match data.host.get_caller() {
        Ok(c) => c,
        Err(_) => return -1,
    };

    match write_memory(data, &mut env.as_store_mut(), ptr, caller.as_bytes()) {
        Ok(()) => 20,
        Err(_) => -1,
    }
}

fn host_get_address(mut env: FunctionEnvMut<WasmEnv>, ptr: u32) -> i32 {
    let data = env.data_mut();
    let address = match data.host.get_address() {
        Ok(a) => a,
        Err(_) => return -1,
    };

    match write_memory(data, &mut env.as_store_mut(), ptr, address.as_bytes()) {
        Ok(()) => 20,
        Err(_) => -1,
    }
}

fn host_get_value(env: FunctionEnvMut<WasmEnv>) -> u64 {
    let data = env.data();
    // Truncate to u64 for simplicity (full Amount is u128)
    data.host.get_value() as u64
}

fn host_get_input_len(env: FunctionEnvMut<WasmEnv>) -> u32 {
    env.data().host.get_input().len() as u32
}

fn host_get_input(mut env: FunctionEnvMut<WasmEnv>, ptr: u32) -> i32 {
    let input = env.data().host.get_input().to_vec();
    let data = env.data_mut();

    match write_memory(data, &mut env.as_store_mut(), ptr, &input) {
        Ok(()) => input.len() as i32,
        Err(_) => -1,
    }
}

fn host_get_block_height(env: FunctionEnvMut<WasmEnv>) -> u64 {
    env.data().host.get_block_height().unwrap_or(0)
}

fn host_get_block_timestamp(env: FunctionEnvMut<WasmEnv>) -> u64 {
    env.data().host.get_block_timestamp().unwrap_or(0)
}

fn host_sha3(mut env: FunctionEnvMut<WasmEnv>, data_ptr: u32, data_len: u32, out_ptr: u32) -> i32 {
    let (wasm_env, store) = env.data_and_store_mut();

    let input = match read_memory(wasm_env, &store, data_ptr, data_len) {
        Ok(d) => d,
        Err(_) => return -1,
    };

    let hash = match wasm_env.host.sha3(&input) {
        Ok(h) => h,
        Err(_) => return -1,
    };

    match write_memory(wasm_env, &mut env.as_store_mut(), out_ptr, &hash) {
        Ok(()) => 32,
        Err(_) => -1,
    }
}

fn host_verify_signature(
    mut env: FunctionEnvMut<WasmEnv>,
    msg_ptr: u32,
    msg_len: u32,
    sig_ptr: u32,
    sig_len: u32,
    pk_ptr: u32,
    pk_len: u32,
) -> i32 {
    let (data, store) = env.data_and_store_mut();

    let message = match read_memory(data, &store, msg_ptr, msg_len) {
        Ok(m) => m,
        Err(_) => return -1,
    };

    let signature = match read_memory(data, &store, sig_ptr, sig_len) {
        Ok(s) => s,
        Err(_) => return -1,
    };

    let public_key = match read_memory(data, &store, pk_ptr, pk_len) {
        Ok(p) => p,
        Err(_) => return -1,
    };

    match data.host.verify_signature(&message, &signature, &public_key) {
        Ok(true) => 1,
        Ok(false) => 0,
        Err(_) => -1,
    }
}

fn host_set_return(mut env: FunctionEnvMut<WasmEnv>, ptr: u32, len: u32) -> i32 {
    let (data, store) = env.data_and_store_mut();

    let return_data = match read_memory(data, &store, ptr, len) {
        Ok(d) => d,
        Err(_) => return -1,
    };

    data.return_data = return_data;
    0
}

fn host_emit_log(
    mut env: FunctionEnvMut<WasmEnv>,
    topics_ptr: u32,
    topics_count: u32,
    data_ptr: u32,
    data_len: u32,
) -> i32 {
    let (wasm_env, store) = env.data_and_store_mut();

    // Read topics (each topic is 32 bytes)
    let mut topics = Vec::with_capacity(topics_count as usize);
    for i in 0..topics_count {
        let offset = topics_ptr + i * 32;
        let topic_bytes = match read_memory(wasm_env, &store, offset, 32) {
            Ok(t) => t,
            Err(_) => return -1,
        };
        let mut topic = [0u8; 32];
        topic.copy_from_slice(&topic_bytes);
        topics.push(topic);
    }

    let data = match read_memory(wasm_env, &store, data_ptr, data_len) {
        Ok(d) => d,
        Err(_) => return -1,
    };

    match wasm_env.host.emit_log(topics, data) {
        Ok(()) => 0,
        Err(_) => -1,
    }
}

fn host_abort(env: FunctionEnvMut<WasmEnv>, msg_ptr: u32, msg_len: u32) {
    let (data, store) = env.data_and_store_mut();

    if let Ok(msg_bytes) = read_memory(data, &store, msg_ptr, msg_len) {
        let msg = String::from_utf8_lossy(&msg_bytes);
        tracing::error!(message = %msg, "Contract aborted");
    }
    
    // This will cause the WASM execution to trap
    panic!("contract aborted");
}

#[cfg(test)]
mod tests {
    use super::*;

    // A minimal WASM module that does nothing (for testing instantiation)
    const MINIMAL_WASM: &[u8] = &[
        0x00, 0x61, 0x73, 0x6d, // magic number
        0x01, 0x00, 0x00, 0x00, // version
    ];

    #[test]
    fn test_runtime_creation() {
        let runtime = Runtime::new();
        assert!(runtime.module_cache.is_empty());
    }

    #[test]
    fn test_balance_operations() {
        let mut runtime = Runtime::new();
        let addr = Address::from_bytes([1u8; 20]);

        runtime.set_balance(&addr, 1000);
        assert_eq!(runtime.get_balance(&addr), 1000);
    }

    #[test]
    fn test_contract_not_found() {
        let mut runtime = Runtime::new();
        let addr = Address::from_bytes([1u8; 20]);
        let caller = Address::from_bytes([2u8; 20]);

        let result = runtime.call(addr, caller, 0, vec![], 100000, BlockContext::default());
        assert!(matches!(result, Err(VmError::ContractNotFound(_))));
    }
}
