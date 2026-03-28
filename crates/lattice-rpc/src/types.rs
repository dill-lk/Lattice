//! JSON-RPC 2.0 type definitions

use serde::{Deserialize, Serialize};

/// JSON-RPC 2.0 request
#[derive(Debug, Clone, Deserialize)]
pub struct RpcRequest {
    pub jsonrpc: String,
    pub method: String,
    #[serde(default)]
    pub params: serde_json::Value,
    pub id: serde_json::Value,
}

impl RpcRequest {
    /// Validate the request format
    pub fn validate(&self) -> bool {
        self.jsonrpc == "2.0"
    }
}

/// JSON-RPC 2.0 response
#[derive(Debug, Clone, Serialize)]
pub struct RpcResponse {
    pub jsonrpc: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub result: Option<serde_json::Value>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<crate::error::RpcError>,
    pub id: serde_json::Value,
}

impl RpcResponse {
    pub fn success(id: serde_json::Value, result: serde_json::Value) -> Self {
        Self {
            jsonrpc: "2.0",
            result: Some(result),
            error: None,
            id,
        }
    }

    pub fn error(id: serde_json::Value, error: crate::error::RpcError) -> Self {
        Self {
            jsonrpc: "2.0",
            result: None,
            error: Some(error),
            id,
        }
    }
}

/// Block number parameter - can be number or tag
#[derive(Debug, Clone, Deserialize)]
#[serde(untagged)]
pub enum BlockNumber {
    Number(u64),
    Tag(BlockTag),
}

/// Block tags for special block references
#[derive(Debug, Clone, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum BlockTag {
    Latest,
    Earliest,
    Pending,
}

/// Call request for lat_call and lat_estimateGas
#[derive(Debug, Clone, Deserialize)]
pub struct CallRequest {
    pub from: Option<String>,
    pub to: String,
    #[serde(default)]
    pub data: Option<String>,
    #[serde(default)]
    pub value: Option<String>,
    #[serde(default)]
    pub gas: Option<u64>,
}

/// Transaction receipt
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct TransactionReceipt {
    pub transaction_hash: String,
    pub block_hash: String,
    pub block_number: String,
    pub from: String,
    pub to: String,
    pub gas_used: String,
    pub status: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub contract_address: Option<String>,
}

/// Serialized block for RPC responses
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcBlock {
    pub number: String,
    pub hash: String,
    pub parent_hash: String,
    pub timestamp: String,
    pub difficulty: String,
    pub nonce: String,
    pub miner: String,
    pub transactions_root: String,
    pub state_root: String,
    pub transactions: Vec<serde_json::Value>,
}

/// Serialized transaction for RPC responses
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct RpcTransaction {
    pub hash: String,
    pub from: String,
    pub to: String,
    pub value: String,
    pub gas: String,
    pub nonce: String,
    pub input: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub block_number: Option<String>,
}
