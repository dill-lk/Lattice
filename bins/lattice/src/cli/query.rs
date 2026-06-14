//! Query command handlers.

use anyhow::{anyhow, Result};
use lattice_core::Address;
use serde_json::Value;

use crate::cli::formatter;
use crate::cli::output;
use crate::cli::rpc_client::RpcClient;

/// Get block by number or hash.
pub async fn get_block(id: &str, include_txs: bool, rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    let block: Value = if id.starts_with("0x") && id.len() == 66 {
        client.get_block_by_hash(id, include_txs).await?
    } else if id == "latest" {
        let height = client.get_block_number().await?;
        client.get_block_by_number(height, include_txs).await?
    } else if matches!(id, "earliest" | "genesis") {
        client.get_block_by_number(0, include_txs).await?
    } else {
        let height: u64 = if id.starts_with("0x") {
            u64::from_str_radix(id.trim_start_matches("0x"), 16)
                .map_err(|_| anyhow!("Invalid block number"))?
        } else {
            id.parse().map_err(|_| anyhow!("Invalid block number"))?
        };
        client.get_block_by_number(height, include_txs).await?
    };

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "query_block",
            "block": block,
        }));
    }

    print_block(&block, include_txs);
    Ok(())
}

fn print_block(block: &Value, include_txs: bool) {
    let number_str = block.get("number").and_then(|v| v.as_str()).unwrap_or("0x0");
    let height = parse_hex_u64(number_str).unwrap_or(0);
    let hash_str = block.get("hash").and_then(|v| v.as_str()).unwrap_or("0x00");

    let mut hash = [0u8; 32];
    if let Ok(bytes) = hex::decode(hash_str.trim_start_matches("0x")) {
        if bytes.len() == 32 {
            hash.copy_from_slice(&bytes);
        }
    }

    let timestamp = block
        .get("timestamp")
        .and_then(|v| v.as_str())
        .map(parse_hex_u64)
        .transpose()
        .ok()
        .flatten()
        .unwrap_or(0);

    let miner = block
        .get("miner")
        .and_then(|v| v.as_str())
        .and_then(|s| Address::from_base58(s).ok())
        .unwrap_or_else(Address::zero);

    let txs = block
        .get("transactions")
        .and_then(|v| v.as_array())
        .cloned()
        .unwrap_or_default();

    formatter::print_block_card(height, &hash, timestamp, txs.len(), &miner);

    if let Some(parent_hash) = block.get("parentHash").and_then(|v| v.as_str()) {
        formatter::key_value("Parent Hash", parent_hash);
    }
    if let Some(difficulty) = block.get("difficulty").and_then(|v| v.as_str()) {
        formatter::key_value("Difficulty", difficulty);
    }
    if let Some(nonce) = block.get("nonce").and_then(|v| v.as_str()) {
        formatter::key_value("Nonce", nonce);
    }

    if include_txs {
        if txs.is_empty() {
            formatter::note("This block has no transactions.");
        } else {
            formatter::subheader("Transactions");
            for (index, tx) in txs.iter().enumerate() {
                if let Some(hash) = tx.as_str() {
                    println!("  [{index}] {hash}");
                    continue;
                }

                let hash = tx.get("hash").and_then(|v| v.as_str()).unwrap_or("<unknown>");
                let from = tx.get("from").and_then(|v| v.as_str()).unwrap_or("?");
                let to = tx.get("to").and_then(|v| v.as_str()).unwrap_or("?");
                println!("  [{index}] {hash}");
                println!("       from: {from}");
                println!("       to:   {to}");
            }
        }
    }

    println!();
}

/// Get transaction by hash.
pub async fn get_transaction(hash: &str, rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);
    let hash_clean = if hash.starts_with("0x") {
        hash.to_string()
    } else {
        format!("0x{hash}")
    };

    match client.get_transaction(&hash_clean).await? {
        Some(tx) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": true,
                    "action": "query_transaction",
                    "transaction": tx,
                }));
            }
            let mut tx_hash = [0u8; 32];
            if let Ok(bytes) = hex::decode(hash_clean.trim_start_matches("0x")) {
                if bytes.len() == 32 {
                    tx_hash.copy_from_slice(&bytes);
                }
            }

            let from = tx
                .get("from")
                .and_then(|v| v.as_str())
                .and_then(|s| Address::from_base58(s).ok())
                .unwrap_or_else(Address::zero);
            let to = tx
                .get("to")
                .and_then(|v| v.as_str())
                .and_then(|s| Address::from_base58(s).ok())
                .unwrap_or_else(Address::zero);
            let amount = tx
                .get("value")
                .and_then(|v| v.as_str())
                .map(parse_hex_u128)
                .transpose()?
                .unwrap_or(0);

            let mut status = "Pending";
            let mut block_height = 0;

            if tx.get("blockHash").is_some_and(|v| !v.is_null()) {
                status = "Confirmed";
                if let Some(number) = tx.get("blockNumber").and_then(|v| v.as_str()) {
                    block_height = parse_hex_u64(number).unwrap_or(0);
                }
            }

            if let Ok(Some(receipt)) = client.get_transaction_receipt(&hash_clean).await {
                if let Some(exec) = receipt.get("status").and_then(|v| v.as_str()) {
                    status = if exec == "0x1" { "Success" } else { "Failed" };
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
                    let input_size = input.trim_start_matches("0x").len() / 2;
                    formatter::key_value("Input Data", &format!("{input_size} bytes"));
                }
            }
            println!();
        }
        None => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "query_transaction",
                    "hash": hash_clean,
                    "error": "transaction not found",
                }));
            }
            formatter::error(&format!("Transaction not found: {hash}"));
        }
    }

    Ok(())
}

pub async fn get_contract_state(address: &str, data: Option<&str>, rpc_url: &str) -> Result<()> {
    Address::from_base58(address).map_err(|_| anyhow!("Invalid contract address format"))?;
    let client = RpcClient::new(rpc_url);
    let result = client.call_contract(address, data).await?;

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "query_contract",
            "address": address,
            "data": data,
            "result": result,
        }));
    }

    formatter::title("Contract State Query");
    formatter::divider();
    formatter::key_value("Contract", address);
    formatter::key_value("Result", &result);
    if let Some(payload) = data {
        formatter::key_value("Data", payload);
    }
    println!();
    Ok(())
}

/// Get account information.
pub async fn get_account(address: &str, rpc_url: &str) -> Result<()> {
    let addr = Address::from_base58(address).map_err(|_| anyhow!("Invalid address format"))?;
    let client = RpcClient::new(rpc_url);

    let balance = match client.get_balance(address).await {
        Ok(value) => value,
        Err(e) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "query_account",
                    "address": address,
                    "error": e.to_string(),
                }));
            }
            formatter::error(&format!("Failed to fetch balance: {e}"));
            0
        }
    };

    let nonce = client.get_transaction_count(address).await.unwrap_or(0);

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "query_account",
            "address": address,
            "balance_latt": balance,
            "nonce": nonce,
        }));
    }

    formatter::print_wallet_card(&addr, balance, nonce);
    Ok(())
}

fn parse_hex_u64(s: &str) -> Result<u64> {
    u64::from_str_radix(s.trim_start_matches("0x"), 16)
        .map_err(|e| anyhow!("Invalid hex number: {e}"))
}

fn parse_hex_u128(s: &str) -> Result<u128> {
    u128::from_str_radix(s.trim_start_matches("0x"), 16)
        .map_err(|e| anyhow!("Invalid hex number: {e}"))
}
