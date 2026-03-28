//! RPC client for communicating with Lattice node

use anyhow::{anyhow, Result};
use serde::{de::DeserializeOwned, Deserialize, Serialize};
use serde_json::{json, Value};

/// JSON-RPC 2.0 request
#[derive(Debug, Serialize)]
struct RpcRequest {
    jsonrpc: &'static str,
    method: String,
    params: Value,
    id: u64,
}

/// JSON-RPC 2.0 response
#[derive(Debug, Deserialize)]
struct RpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    result: Option<Value>,
    error: Option<RpcError>,
    #[allow(dead_code)]
    id: u64,
}

/// JSON-RPC 2.0 error
#[derive(Debug, Deserialize)]
struct RpcError {
    code: i64,
    message: String,
    #[allow(dead_code)]
    data: Option<Value>,
}

/// RPC client for Lattice node
pub struct RpcClient {
    endpoint: String,
    client: reqwest::Client,
    request_id: std::sync::atomic::AtomicU64,
}

impl RpcClient {
    /// Create a new RPC client
    pub fn new(endpoint: &str) -> Self {
        Self {
            endpoint: endpoint.to_string(),
            client: reqwest::Client::new(),
            request_id: std::sync::atomic::AtomicU64::new(1),
        }
    }

    /// Get the next request ID
    fn next_id(&self) -> u64 {
        self.request_id
            .fetch_add(1, std::sync::atomic::Ordering::SeqCst)
    }

    /// Make an RPC call
    pub async fn call<T: DeserializeOwned>(&self, method: &str, params: Value) -> Result<T> {
        let request = RpcRequest {
            jsonrpc: "2.0",
            method: method.to_string(),
            params,
            id: self.next_id(),
        };

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "HTTP error: {} {}",
                response.status().as_u16(),
                response.status().as_str()
            ));
        }

        let rpc_response: RpcResponse = response.json().await?;

        if let Some(error) = rpc_response.error {
            return Err(anyhow!("RPC error ({}): {}", error.code, error.message));
        }

        match rpc_response.result {
            Some(result) => {
                serde_json::from_value(result).map_err(|e| anyhow!("Failed to parse result: {}", e))
            }
            None => Err(anyhow!("No result in response")),
        }
    }

    /// Get current block number
    pub async fn get_block_number(&self) -> Result<u64> {
        let result: String = self.call("lat_blockNumber", json!([])).await?;
        parse_hex_u64(&result)
    }

    /// Get balance for an address
    pub async fn get_balance(&self, address: &str) -> Result<u128> {
        let result: String = self.call("lat_getBalance", json!([address])).await?;
        parse_hex_u128(&result)
    }

    /// Get block by number
    pub async fn get_block_by_number(&self, number: u64, include_txs: bool) -> Result<Value> {
        self.call(
            "lat_getBlockByNumber",
            json!([format!("0x{:x}", number), include_txs]),
        )
        .await
    }

    /// Get block by hash
    pub async fn get_block_by_hash(&self, hash: &str, include_txs: bool) -> Result<Value> {
        self.call("lat_getBlockByHash", json!([hash, include_txs]))
            .await
    }

    /// Get transaction by hash
    pub async fn get_transaction(&self, hash: &str) -> Result<Option<Value>> {
        let result: Value = self.call("lat_getTransactionByHash", json!([hash])).await?;
        if result.is_null() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    /// Get transaction receipt
    pub async fn get_transaction_receipt(&self, hash: &str) -> Result<Option<Value>> {
        let result: Value = self
            .call("lat_getTransactionReceipt", json!([hash]))
            .await?;
        if result.is_null() {
            Ok(None)
        } else {
            Ok(Some(result))
        }
    }

    /// Send raw transaction
    pub async fn send_raw_transaction(&self, tx_hex: &str) -> Result<String> {
        self.call("lat_sendRawTransaction", json!([tx_hex])).await
    }

    /// Get account nonce (transaction count)
    pub async fn get_transaction_count(&self, address: &str) -> Result<u64> {
        // This would be a real RPC call in production
        // For now, we simulate it
        let result: String = self
            .call("lat_getTransactionCount", json!([address, "latest"]))
            .await
            .unwrap_or_else(|_| "0x0".to_string());
        parse_hex_u64(&result)
    }

    /// Get node sync status
    pub async fn get_sync_status(&self) -> Result<SyncStatus> {
        let block_number = self.get_block_number().await?;
        // In a real implementation, this would query actual sync status
        Ok(SyncStatus {
            syncing: false,
            current_block: block_number,
            highest_block: block_number,
        })
    }

    /// Get connected peers
    pub async fn get_peers(&self) -> Result<Vec<PeerInfo>> {
        // In a real implementation, this would query net_peerCount or similar
        // For now return empty list
        Ok(vec![])
    }
}

/// Sync status information
#[derive(Debug)]
pub struct SyncStatus {
    pub syncing: bool,
    pub current_block: u64,
    pub highest_block: u64,
}

/// Peer information
#[derive(Debug)]
pub struct PeerInfo {
    pub id: String,
    pub address: String,
    pub latency_ms: u64,
}

/// Parse hex string to u64
fn parse_hex_u64(s: &str) -> Result<u64> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(s, 16).map_err(|e| anyhow!("Invalid hex number: {}", e))
}

/// Parse hex string to u128
fn parse_hex_u128(s: &str) -> Result<u128> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    u128::from_str_radix(s, 16).map_err(|e| anyhow!("Invalid hex number: {}", e))
}
