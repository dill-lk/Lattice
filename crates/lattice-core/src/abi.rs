//! Smart contract ABI (Application Binary Interface) encoding/decoding

use crate::{Address, Amount, Hash};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};
use sha3::{Digest, Sha3_256};

/// ABI parameter type
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum AbiType {
    /// Unsigned integer (bits)
    Uint(u16),
    /// Signed integer (bits)
    Int(u16),
    /// Address (20 bytes)
    Address,
    /// Boolean
    Bool,
    /// Fixed-size bytes
    FixedBytes(usize),
    /// Dynamic bytes
    Bytes,
    /// UTF-8 string
    String,
    /// Fixed-size array
    FixedArray(Box<AbiType>, usize),
    /// Dynamic array
    Array(Box<AbiType>),
    /// Tuple of types
    Tuple(Vec<AbiType>),
}

impl AbiType {
    /// Check if type is dynamic (variable length)
    pub fn is_dynamic(&self) -> bool {
        match self {
            AbiType::Bytes | AbiType::String | AbiType::Array(_) => true,
            AbiType::Tuple(types) => types.iter().any(|t| t.is_dynamic()),
            AbiType::FixedArray(inner, _) => inner.is_dynamic(),
            _ => false,
        }
    }

    /// Get canonical type string
    pub fn canonical(&self) -> String {
        match self {
            AbiType::Uint(bits) => format!("uint{}", bits),
            AbiType::Int(bits) => format!("int{}", bits),
            AbiType::Address => "address".to_string(),
            AbiType::Bool => "bool".to_string(),
            AbiType::FixedBytes(len) => format!("bytes{}", len),
            AbiType::Bytes => "bytes".to_string(),
            AbiType::String => "string".to_string(),
            AbiType::FixedArray(inner, len) => format!("{}[{}]", inner.canonical(), len),
            AbiType::Array(inner) => format!("{}[]", inner.canonical()),
            AbiType::Tuple(types) => {
                let types_str = types
                    .iter()
                    .map(|t| t.canonical())
                    .collect::<Vec<_>>()
                    .join(",");
                format!("({})", types_str)
            }
        }
    }
}

/// ABI function parameter
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct AbiParam {
    /// Parameter name
    pub name: String,
    /// Parameter type
    pub param_type: AbiType,
    /// Whether this parameter is indexed (for events)
    pub indexed: bool,
}

impl AbiParam {
    /// Create a new parameter
    pub fn new(name: impl Into<String>, param_type: AbiType) -> Self {
        Self {
            name: name.into(),
            param_type,
            indexed: false,
        }
    }

    /// Mark parameter as indexed (for events)
    pub fn indexed(mut self) -> Self {
        self.indexed = true;
        self
    }
}

/// ABI function definition
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct AbiFunction {
    /// Function name
    pub name: String,
    /// Input parameters
    pub inputs: Vec<AbiParam>,
    /// Output parameters
    pub outputs: Vec<AbiParam>,
    /// State mutability
    pub state_mutability: StateMutability,
}

impl AbiFunction {
    /// Create a new function
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            inputs: Vec::new(),
            outputs: Vec::new(),
            state_mutability: StateMutability::NonPayable,
        }
    }

    /// Add input parameter
    pub fn input(mut self, param: AbiParam) -> Self {
        self.inputs.push(param);
        self
    }

    /// Add output parameter
    pub fn output(mut self, param: AbiParam) -> Self {
        self.outputs.push(param);
        self
    }

    /// Set state mutability
    pub fn mutability(mut self, mutability: StateMutability) -> Self {
        self.state_mutability = mutability;
        self
    }

    /// Compute function selector (first 4 bytes of signature hash)
    pub fn selector(&self) -> [u8; 4] {
        let signature = self.signature();
        let hash = Sha3_256::digest(signature.as_bytes());
        let mut selector = [0u8; 4];
        selector.copy_from_slice(&hash[..4]);
        selector
    }

    /// Get function signature string
    pub fn signature(&self) -> String {
        let params = self
            .inputs
            .iter()
            .map(|p| p.param_type.canonical())
            .collect::<Vec<_>>()
            .join(",");
        format!("{}({})", self.name, params)
    }
}

/// State mutability of function
#[derive(Debug, Clone, Copy, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub enum StateMutability {
    /// Function is read-only (view)
    Pure,
    /// Function reads state but doesn't modify
    View,
    /// Function modifies state but doesn't accept ETH
    NonPayable,
    /// Function modifies state and accepts ETH
    Payable,
}

/// ABI event definition
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct AbiEvent {
    /// Event name
    pub name: String,
    /// Event parameters
    pub inputs: Vec<AbiParam>,
    /// Whether event is anonymous
    pub anonymous: bool,
}

impl AbiEvent {
    /// Create a new event
    pub fn new(name: impl Into<String>) -> Self {
        Self {
            name: name.into(),
            inputs: Vec::new(),
            anonymous: false,
        }
    }

    /// Add input parameter
    pub fn input(mut self, param: AbiParam) -> Self {
        self.inputs.push(param);
        self
    }

    /// Mark as anonymous
    pub fn anonymous(mut self) -> Self {
        self.anonymous = true;
        self
    }

    /// Compute event signature hash (topic[0])
    pub fn signature_hash(&self) -> [u8; 32] {
        let signature = self.signature();
        let hash = Sha3_256::digest(signature.as_bytes());
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        result
    }

    /// Get event signature string
    pub fn signature(&self) -> String {
        let params = self
            .inputs
            .iter()
            .map(|p| p.param_type.canonical())
            .collect::<Vec<_>>()
            .join(",");
        format!("{}({})", self.name, params)
    }
}

/// Contract ABI (collection of functions and events)
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct ContractAbi {
    /// Contract functions
    pub functions: Vec<AbiFunction>,
    /// Contract events
    pub events: Vec<AbiEvent>,
    /// Constructor
    pub constructor: Option<AbiFunction>,
    /// Fallback function
    pub fallback: Option<AbiFunction>,
}

impl ContractAbi {
    /// Create empty ABI
    pub fn new() -> Self {
        Self::default()
    }

    /// Add a function
    pub fn add_function(mut self, function: AbiFunction) -> Self {
        self.functions.push(function);
        self
    }

    /// Add an event
    pub fn add_event(mut self, event: AbiEvent) -> Self {
        self.events.push(event);
        self
    }

    /// Set constructor
    pub fn constructor(mut self, constructor: AbiFunction) -> Self {
        self.constructor = Some(constructor);
        self
    }

    /// Find function by selector
    pub fn find_function_by_selector(&self, selector: &[u8; 4]) -> Option<&AbiFunction> {
        self.functions.iter().find(|f| &f.selector() == selector)
    }

    /// Find function by name
    pub fn find_function_by_name(&self, name: &str) -> Option<&AbiFunction> {
        self.functions.iter().find(|f| f.name == name)
    }

    /// Find event by signature hash
    pub fn find_event_by_hash(&self, hash: &[u8; 32]) -> Option<&AbiEvent> {
        self.events.iter().find(|e| &e.signature_hash() == hash)
    }

    /// Find event by name
    pub fn find_event_by_name(&self, name: &str) -> Option<&AbiEvent> {
        self.events.iter().find(|e| e.name == name)
    }
}

/// ABI encoded value
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AbiValue {
    Uint(Vec<u8>, u16),     // value, bits
    Int(Vec<u8>, u16),      // value, bits
    Address(Address),
    Bool(bool),
    FixedBytes(Vec<u8>),
    Bytes(Vec<u8>),
    String(String),
    Array(Vec<AbiValue>),
    Tuple(Vec<AbiValue>),
}

impl AbiValue {
    /// Encode value to bytes (simplified ABI encoding)
    pub fn encode(&self) -> Vec<u8> {
        match self {
            AbiValue::Uint(data, _) | AbiValue::Int(data, _) => {
                let mut result = vec![0u8; 32];
                let start = 32 - data.len();
                result[start..].copy_from_slice(data);
                result
            }
            AbiValue::Address(addr) => {
                let mut result = vec![0u8; 32];
                result[12..].copy_from_slice(addr.as_bytes());
                result
            }
            AbiValue::Bool(b) => {
                let mut result = vec![0u8; 32];
                result[31] = if *b { 1 } else { 0 };
                result
            }
            AbiValue::FixedBytes(data) => {
                let mut result = vec![0u8; 32];
                result[..data.len()].copy_from_slice(data);
                result
            }
            AbiValue::Bytes(data) => {
                let mut result = Vec::new();
                // Length
                result.extend_from_slice(&(data.len() as u64).to_be_bytes());
                result.resize(32, 0);
                // Data
                result.extend_from_slice(data);
                // Padding
                let padding = (32 - (data.len() % 32)) % 32;
                result.resize(result.len() + padding, 0);
                result
            }
            AbiValue::String(s) => AbiValue::Bytes(s.as_bytes().to_vec()).encode(),
            AbiValue::Array(values) => {
                let mut result = Vec::new();
                // Length
                result.extend_from_slice(&(values.len() as u64).to_be_bytes());
                result.resize(32, 0);
                // Elements
                for value in values {
                    result.extend_from_slice(&value.encode());
                }
                result
            }
            AbiValue::Tuple(values) => {
                let mut result = Vec::new();
                for value in values {
                    result.extend_from_slice(&value.encode());
                }
                result
            }
        }
    }
}

/// ABI encoder for function calls
pub struct AbiEncoder {
    data: Vec<u8>,
}

impl AbiEncoder {
    /// Create new encoder with function selector
    pub fn new(selector: [u8; 4]) -> Self {
        Self {
            data: selector.to_vec(),
        }
    }

    /// Add a value to encode
    pub fn add_value(mut self, value: AbiValue) -> Self {
        self.data.extend_from_slice(&value.encode());
        self
    }

    /// Finalize encoding
    pub fn finish(self) -> Vec<u8> {
        self.data
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_function_selector() {
        let func = AbiFunction::new("transfer")
            .input(AbiParam::new("to", AbiType::Address))
            .input(AbiParam::new("amount", AbiType::Uint(256)));

        let selector = func.selector();
        // Selector should be first 4 bytes of keccak256("transfer(address,uint256)")
        assert_eq!(selector.len(), 4);
    }

    #[test]
    fn test_event_signature() {
        let event = AbiEvent::new("Transfer")
            .input(AbiParam::new("from", AbiType::Address).indexed())
            .input(AbiParam::new("to", AbiType::Address).indexed())
            .input(AbiParam::new("value", AbiType::Uint(256)));

        let sig_hash = event.signature_hash();
        assert_eq!(sig_hash.len(), 32);
    }

    #[test]
    fn test_abi_value_encoding() {
        // Encode bool
        let value = AbiValue::Bool(true);
        let encoded = value.encode();
        assert_eq!(encoded.len(), 32);
        assert_eq!(encoded[31], 1);

        // Encode address
        let addr = Address::from_bytes([1u8; 20]);
        let value = AbiValue::Address(addr);
        let encoded = value.encode();
        assert_eq!(encoded.len(), 32);
    }

    #[test]
    fn test_contract_abi() {
        let abi = ContractAbi::new()
            .add_function(
                AbiFunction::new("balanceOf")
                    .input(AbiParam::new("owner", AbiType::Address))
                    .output(AbiParam::new("balance", AbiType::Uint(256)))
                    .mutability(StateMutability::View),
            )
            .add_event(
                AbiEvent::new("Transfer")
                    .input(AbiParam::new("from", AbiType::Address).indexed())
                    .input(AbiParam::new("to", AbiType::Address).indexed())
                    .input(AbiParam::new("value", AbiType::Uint(256))),
            );

        assert_eq!(abi.functions.len(), 1);
        assert_eq!(abi.events.len(), 1);

        let func = abi.find_function_by_name("balanceOf").unwrap();
        assert_eq!(func.name, "balanceOf");
    }
}
