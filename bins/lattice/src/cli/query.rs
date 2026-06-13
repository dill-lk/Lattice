//! Query command handlers

use anyhow::{anyhow, Result};
use lattice_core::Address;
use serde_json::Value;

use crate::cli::rpc_client::RpcClient;
use crate::cli::formatter;

/// Get block by number or hash
pub async fn get_block(id: &str, include_txs: bool, rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    let block: Value = if id.starts_with("0x") && id.len() == 66 {
        client.get_block_by_hash(id, include_txs).await?
    } else if id == "latest" {
        let height = client.get_block_number().await?;
        client.get_block_by_number(height, include_txs).await?
    } else if id == "earliest" || id == "genesis" {
        client.get_block_by_number(0, include_txs).await?
    } else {
        let height: u64 = if id.starts_with("0x") {
            u64::from_str_radix(id.strip_prefix("0x").unwrap(), 16)
                .map_err(|_| anyhow!("Invalid block number"))?
        } else {
            id.parse().map_err(|_| anyhow!("Invalid block number"))?
        };
        client.get_block_by_number(height, include_txs).await?
    };

    print_block(&block, include_txs);
    Ok(())
}

/// Print block details
fn print_block(block: &Value, include_txs: bool) {
    let number_str = block.get("number").and_then(|v| v.as_str()).unwrap_or("0x0");
    let height = parse_hex_u64(number_str).unwrap_or(0);
    let hash_str = block.get("hash").and_then(|v| v.as_str()).unwrap_or("0x00");
    
    let mut hash = [0u8; 32];
    if let Ok(bytes) = hex::decode(hash_str.strip_prefix("0x").unwrap_or(hash_str)) {
        if bytes.len() == 32 {
            hash.copy_from_slice(&bytes);
        }
    }

    let timestamp_str = block.get("timestamp").and_then(|v| v.as_str()).unwrap_or("0x0");
    let timestamp = parse_hex_u64(timestamp_str).unwrap_or(0);
    
    let miner_str = block.get("miner").and_then(|v| v.as_str()).unwrap_or("");
    let miner = Address::from_base58(miner_str).unwrap_or_else(|_| Address::zero());

    let txs_count = block
        .get("transactions")
        .and_then(|v| v.as_array())
        .map_or(0, |a| a.len());

    formatter::print_block_card(height, &hash, timestamp, txs_count, &miner);

    if let Some(parent) = block.get("parentHash").and_then(|v| v.as_str()) {
        formatter::key_value("Parent Hash", parent);
    }
    if let Some(difficulty) = block.get("difficulty").and_then(|v| v.as_str()) {
        formatter::key_value("Difficulty", difficulty);
    }
    if let Some(nonce) = block.get("nonce").and_then(|v| v.as_str()) {
        formatter::key_value("Nonce", nonce);
    }

    if let Some(txs) = block.get("transactions").and_then(|v| v.as_array()) {
        if include_txs && !txs.is_empty() {
            println!();
            formatter::subheader("Block Transactions");
            for (i, tx) in txs.iter().enumerate() {
                if let Some(h) = tx.as_str() {
                    println!("  [{}] {}", i, h);
                } else if let Some(h) = tx.get("hash").and_then(|v| v.as_str()) {
                    let from = tx.get("from").and_then(|v| v.as_str()).unwrap_or("?");
                    let to = tx.get("to").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("  [{}] {} -> {} (hash: {})", i, from, to, h);
                }
            }
        }
    }
    println!();
}

/// Get transaction by hash
pub async fn get_transaction(hash: &str, rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    let hash_clean = if hash.starts_with("0x") {
        hash.to_string()
    } else {
        format!("0x{}", hash)
    };

    match client.get_transaction(&hash_clean).await? {
        Some(tx) => {
            let mut tx_hash = [0u8; 32];
            if let Ok(bytes) = hex::decode(hash_clean.strip_prefix("0x").unwrap_or(&hash_clean)) {
                if bytes.len() == 32 {
                    tx_hash.copy_from_slice(&bytes);
                }
            }

            let from_str = tx.get("from").and_then(|v| v.as_str()).unwrap_or("");
            let from = Address::from_base58(from_str).unwrap_or_else(|_| Address::zero());
            
            let to_str = tx.get("to").and_then(|v| v.as_str()).unwrap_or("");
            let to = Address::from_base58(to_str).unwrap_or_else(|_| Address::zero());

            let amount_hex = tx.get("value").and_then(|v| v.as_str()).unwrap_or("0x0");
            let amount = parse_hex_u128(amount_hex).unwrap_or(0);

            let mut status = "Pending";
            let mut block_height = 0;

            if let Some(block_hash) = tx.get("blockHash") {
                if !block_hash.is_null() {
                    status = "Confirmed";
                    if let Some(bn) = tx.get("blockNumber").and_then(|v| v.as_str()) {
                        block_height = parse_hex_u64(bn).unwrap_or(0);
                    }
                }
            }

            if let Ok(Some(receipt)) = client.get_transaction_receipt(&hash_clean).await {
                if let Some(st) = receipt.get("status").and_then(|v| v.as_str()) {
                    status = if st == "0x1" { "Success" } else { "Failed" };
                }
            }

            formatter::print_transaction_card(&tx_hash, &from, &to, amount, status, block_height);

            if let Some(gas) = tx.get("gas").and_then(|v| v.as_str()) {
                formatter::key_value("Gas Limit", gas);
            }
            if let Some(nonce) = tx.get("nonce").and_then(|v| v.as_str()) {
                formatter::key_value("Nonce", nonce);
            }
            if let Some(input) = tx.get("input").and_then(|v| v.as_str()) {
                if input != "0x" && !input.is_empty() {
                    let data_len = (input.len() - 2) / 2;
                    formatter::key_value("Input Data Size", &format!("{} bytes", data_len));
                }
            }
            println!();
        }
        None => {
            formatter::error(&format!("Transaction not found: {}", hash));
        }
    }

    Ok(())
}

/// Get account information
pub async fn get_account(address: &str, rpc_url: &str) -> Result<()> {
    let addr = Address::from_base58(address).map_err(|_| anyhow!("Invalid address format"))?;
    let client = RpcClient::new(rpc_url);

    let mut balance = 0;
    let mut nonce = 0;

    match client.get_balance(address).await {
        Ok(bal) => balance = bal,
        Err(e) => formatter::error(&format!("Error fetching balance: {}", e)),
    }

    match client.get_transaction_count(address).await {
        Ok(n) => nonce = n,
        Err(_) => {}
    }

    formatter::print_wallet_card(&addr, balance, nonce);
    Ok(())
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
