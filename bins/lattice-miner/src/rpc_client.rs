//! RPC client for communicating with the Lattice node

use anyhow::{anyhow, Context, Result};
use lattice_core::{Address, BlockHeader, Hash};
use reqwest::Client;
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::sync::atomic::{AtomicU64, Ordering};

/// Work template received from the node
#[derive(Debug, Clone)]
pub struct WorkTemplate {
    /// Block header to mine
    pub header: BlockHeader,
    /// Block transactions (for reference)
    pub tx_count: usize,
    /// Work ID for submission
    pub work_id: String,
}

/// Solution to submit back to the node
#[derive(Debug, Clone, Serialize)]
pub struct WorkSolution {
    /// Work ID from the template
    pub work_id: String,
    /// Found nonce
    pub nonce: u64,
    /// Resulting PoW hash
    pub pow_hash: String,
}

/// JSON-RPC request
#[derive(Debug, Serialize)]
struct RpcRequest {
    jsonrpc: &'static str,
    method: String,
    params: Value,
    id: u64,
}

/// JSON-RPC response
#[derive(Debug, Deserialize)]
struct RpcResponse {
    #[allow(dead_code)]
    jsonrpc: String,
    #[serde(default)]
    result: Option<Value>,
    #[serde(default)]
    error: Option<RpcErrorObj>,
    #[allow(dead_code)]
    id: u64,
}

#[derive(Debug, Deserialize)]
struct RpcErrorObj {
    code: i64,
    message: String,
}

/// RPC client for miner-node communication
pub struct RpcClient {
    client: Client,
    endpoint: String,
    request_id: AtomicU64,
}

impl RpcClient {
    /// Create a new RPC client
    pub fn new(endpoint: &str) -> Result<Self> {
        let client = Client::builder()
            .timeout(std::time::Duration::from_secs(30))
            .build()
            .context("Failed to create HTTP client")?;

        Ok(Self {
            client,
            endpoint: endpoint.to_string(),
            request_id: AtomicU64::new(1),
        })
    }

    /// Send a JSON-RPC request
    async fn call(&self, method: &str, params: Value) -> Result<Value> {
        let id = self.request_id.fetch_add(1, Ordering::SeqCst);

        let request = RpcRequest {
            jsonrpc: "2.0",
            method: method.to_string(),
            params,
            id,
        };

        let response = self
            .client
            .post(&self.endpoint)
            .json(&request)
            .send()
            .await
            .context("Failed to send RPC request")?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "RPC request failed with status: {}",
                response.status()
            ));
        }

        let rpc_response: RpcResponse = response
            .json()
            .await
            .context("Failed to parse RPC response")?;

        if let Some(error) = rpc_response.error {
            return Err(anyhow!("RPC error {}: {}", error.code, error.message));
        }

        rpc_response
            .result
            .ok_or_else(|| anyhow!("RPC response missing result"))
    }

    /// Get work template from the node (lat_getWork)
    ///
    /// Returns a block header template that miners should try to solve.
    pub async fn get_work(&self, coinbase: &Address) -> Result<WorkTemplate> {
        let params = json!([coinbase.to_base58()]);
        let result = self.call("lat_getWork", params).await?;

        // Parse the work template response
        let work_id = result
            .get("workId")
            .and_then(|v| v.as_str())
            .ok_or_else(|| anyhow!("Missing workId in response"))?
            .to_string();

        let header = parse_block_header(&result)?;

        let tx_count = result
            .get("txCount")
            .and_then(|v| v.as_u64())
            .unwrap_or(0) as usize;

        Ok(WorkTemplate {
            header,
            tx_count,
            work_id,
        })
    }

    /// Submit a work solution to the node (lat_submitWork)
    ///
    /// Returns true if the solution was accepted.
    pub async fn submit_work(&self, solution: &WorkSolution) -> Result<bool> {
        let params = json!([{
            "workId": solution.work_id,
            "nonce": format!("0x{:x}", solution.nonce),
            "powHash": solution.pow_hash
        }]);

        let result = self.call("lat_submitWork", params).await?;

        result
            .as_bool()
            .ok_or_else(|| anyhow!("Invalid submitWork response"))
    }

    /// Get the current block number
    #[allow(dead_code)]
    pub async fn block_number(&self) -> Result<u64> {
        let result = self.call("lat_blockNumber", json!([])).await?;

        let num_str = result
            .as_str()
            .ok_or_else(|| anyhow!("Invalid blockNumber response"))?;

        parse_hex_u64(num_str)
    }
}

/// Parse a hex string to u64
fn parse_hex_u64(s: &str) -> Result<u64> {
    let hex_str = s.strip_prefix("0x").unwrap_or(s);
    u64::from_str_radix(hex_str, 16).context("Invalid hex number")
}

/// Parse a block header from JSON response
fn parse_block_header(value: &Value) -> Result<BlockHeader> {
    let header_obj = value
        .get("header")
        .ok_or_else(|| anyhow!("Missing header in work template"))?;

    let version = header_obj
        .get("version")
        .and_then(|v| v.as_u64())
        .unwrap_or(1) as u32;

    let height = header_obj
        .get("height")
        .and_then(|v| v.as_str())
        .map(parse_hex_u64)
        .transpose()?
        .unwrap_or(0);

    let prev_hash = parse_hash(
        header_obj
            .get("prevHash")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0000000000000000000000000000000000000000000000000000000000000000"),
    )?;

    let tx_root = parse_hash(
        header_obj
            .get("txRoot")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0000000000000000000000000000000000000000000000000000000000000000"),
    )?;

    let state_root = parse_hash(
        header_obj
            .get("stateRoot")
            .and_then(|v| v.as_str())
            .unwrap_or("0x0000000000000000000000000000000000000000000000000000000000000000"),
    )?;

    let timestamp = header_obj
        .get("timestamp")
        .and_then(|v| v.as_str())
        .map(parse_hex_u64)
        .transpose()?
        .unwrap_or_else(|| {
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .unwrap()
                .as_millis() as u64
        });

    let difficulty = header_obj
        .get("difficulty")
        .and_then(|v| v.as_str())
        .map(parse_hex_u64)
        .transpose()?
        .unwrap_or(1);

    let coinbase_str = header_obj
        .get("coinbase")
        .and_then(|v| v.as_str())
        .unwrap_or("");

    let coinbase = if coinbase_str.is_empty() {
        Address::zero()
    } else {
        Address::from_base58(coinbase_str).unwrap_or_else(|_| Address::zero())
    };

    Ok(BlockHeader {
        version,
        height,
        prev_hash,
        tx_root,
        state_root,
        timestamp,
        difficulty,
        nonce: 0,
        coinbase,
    })
}

/// Parse a hex-encoded hash
fn parse_hash(s: &str) -> Result<Hash> {
    let hex_str = s.strip_prefix("0x").unwrap_or(s);

    if hex_str.is_empty() || hex_str == "0" {
        return Ok([0u8; 32]);
    }

    // Pad to 64 characters if needed
    let padded = format!("{:0>64}", hex_str);
    let bytes = hex::decode(&padded).context("Invalid hex hash")?;

    if bytes.len() != 32 {
        return Err(anyhow!("Hash must be 32 bytes, got {}", bytes.len()));
    }

    let mut hash = [0u8; 32];
    hash.copy_from_slice(&bytes);
    Ok(hash)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_parse_hex_u64() {
        assert_eq!(parse_hex_u64("0x0").unwrap(), 0);
        assert_eq!(parse_hex_u64("0x1").unwrap(), 1);
        assert_eq!(parse_hex_u64("0xff").unwrap(), 255);
        assert_eq!(parse_hex_u64("0x100").unwrap(), 256);
        assert_eq!(parse_hex_u64("100").unwrap(), 256);
    }

    #[test]
    fn test_parse_hash() {
        let zero_hash = parse_hash("0x0").unwrap();
        assert_eq!(zero_hash, [0u8; 32]);

        let full_hash = parse_hash(
            "0x0000000000000000000000000000000000000000000000000000000000000001",
        )
        .unwrap();
        let mut expected = [0u8; 32];
        expected[31] = 1;
        assert_eq!(full_hash, expected);
    }
}
