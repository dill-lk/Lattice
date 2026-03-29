//! Query command handlers

use anyhow::{anyhow, Result};
use lattice_core::Address;
use serde_json::Value;

use crate::rpc_client::RpcClient;

/// Get block by number or hash
pub async fn get_block(id: &str, include_txs: bool, rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    let block: Value = if id.starts_with("0x") && id.len() == 66 {
        // Looks like a hash (0x + 64 hex chars)
        client.get_block_by_hash(id, include_txs).await?
    } else if id == "latest" {
        let height = client.get_block_number().await?;
        client.get_block_by_number(height, include_txs).await?
    } else if id == "earliest" || id == "genesis" {
        client.get_block_by_number(0, include_txs).await?
    } else {
        // Parse as block number
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
    println!("Block");
    println!("=====");
    println!();

    if let Some(number) = block.get("number").and_then(|v| v.as_str()) {
        if let Ok(n) = parse_hex_u64(number) {
            println!("Number: {} ({})", number, n);
        } else {
            println!("Number: {}", number);
        }
    }

    if let Some(hash) = block.get("hash").and_then(|v| v.as_str()) {
        println!("Hash: {}", hash);
    }

    if let Some(parent) = block.get("parentHash").and_then(|v| v.as_str()) {
        println!("Parent: {}", parent);
    }

    if let Some(miner) = block.get("miner").and_then(|v| v.as_str()) {
        println!("Miner: {}", miner);
    }

    if let Some(timestamp) = block.get("timestamp").and_then(|v| v.as_str()) {
        if let Ok(ts) = parse_hex_u64(timestamp) {
            // Convert ms to datetime
            let secs = ts / 1000;
            let datetime = chrono_format(secs);
            println!("Timestamp: {} ({})", datetime, ts);
        } else {
            println!("Timestamp: {}", timestamp);
        }
    }

    if let Some(difficulty) = block.get("difficulty").and_then(|v| v.as_str()) {
        println!("Difficulty: {}", difficulty);
    }

    if let Some(nonce) = block.get("nonce").and_then(|v| v.as_str()) {
        println!("Nonce: {}", nonce);
    }

    println!();
    println!("Roots");
    println!("-----");

    if let Some(tx_root) = block.get("transactionsRoot").and_then(|v| v.as_str()) {
        println!("Transactions Root: {}", tx_root);
    }

    if let Some(state_root) = block.get("stateRoot").and_then(|v| v.as_str()) {
        println!("State Root: {}", state_root);
    }

    if let Some(txs) = block.get("transactions").and_then(|v| v.as_array()) {
        println!();
        println!("Transactions: {} total", txs.len());

        if include_txs && !txs.is_empty() {
            println!();
            for (i, tx) in txs.iter().enumerate() {
                if let Some(hash) = tx.as_str() {
                    println!("  [{}] {}", i, hash);
                } else if let Some(hash) = tx.get("hash").and_then(|v| v.as_str()) {
                    let from = tx.get("from").and_then(|v| v.as_str()).unwrap_or("?");
                    let to = tx.get("to").and_then(|v| v.as_str()).unwrap_or("?");
                    println!("  [{}] {} -> {}", i, from, to);
                    println!("      Hash: {}", hash);
                }
            }
        }
    }
}

/// Get transaction by hash
pub async fn get_transaction(hash: &str, rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    let hash = if hash.starts_with("0x") {
        hash.to_string()
    } else {
        format!("0x{}", hash)
    };

    match client.get_transaction(&hash).await? {
        Some(tx) => {
            println!("Transaction");
            println!("===========");
            println!();
            println!("Hash: {}", hash);

            if let Some(block_hash) = tx.get("blockHash") {
                if !block_hash.is_null() {
                    println!("Status: Confirmed");
                    if let Some(bn) = tx.get("blockNumber").and_then(|v| v.as_str()) {
                        println!("Block Number: {}", bn);
                    }
                    if let Some(bh) = block_hash.as_str() {
                        println!("Block Hash: {}", bh);
                    }
                } else {
                    println!("Status: Pending");
                }
            }

            println!();

            if let Some(from) = tx.get("from").and_then(|v| v.as_str()) {
                println!("From: {}", from);
            }

            if let Some(to) = tx.get("to").and_then(|v| v.as_str()) {
                println!("To: {}", to);
            }

            if let Some(value) = tx.get("value").and_then(|v| v.as_str()) {
                if let Ok(amount) = parse_hex_u128(value) {
                    println!("Value: {} LAT", format_amount(amount));
                } else {
                    println!("Value: {}", value);
                }
            }

            if let Some(gas) = tx.get("gas").and_then(|v| v.as_str()) {
                println!("Gas Limit: {}", gas);
            }

            if let Some(nonce) = tx.get("nonce").and_then(|v| v.as_str()) {
                println!("Nonce: {}", nonce);
            }

            if let Some(input) = tx.get("input").and_then(|v| v.as_str()) {
                if input != "0x" && !input.is_empty() {
                    let data_len = (input.len() - 2) / 2; // exclude 0x
                    println!("Input Data: {} bytes", data_len);
                }
            }
        }
        None => {
            println!("Transaction not found: {}", hash);
        }
    }

    Ok(())
}

/// Get account information
pub async fn get_account(address: &str, rpc_url: &str) -> Result<()> {
    // Validate address format
    let addr = Address::from_base58(address).map_err(|_| anyhow!("Invalid address format"))?;

    let client = RpcClient::new(rpc_url);

    println!("Account");
    println!("=======");
    println!();
    println!("Address: {}", addr);

    // Get balance
    match client.get_balance(address).await {
        Ok(balance) => {
            println!("Balance: {} LAT", format_amount(balance));
            println!("         ({} wei)", balance);
        }
        Err(e) => {
            println!("Balance: Error fetching - {}", e);
        }
    }

    // Get nonce/transaction count
    match client.get_transaction_count(address).await {
        Ok(nonce) => {
            println!("Nonce: {}", nonce);
        }
        Err(_) => {
            println!("Nonce: Unknown");
        }
    }

    Ok(())
}

/// Format amount in LAT (8 decimals)
fn format_amount(amount: u128) -> String {
    use lattice_core::tokenomics::LATT_PER_LAT;
    
    let whole = amount / LATT_PER_LAT;
    let frac = amount % LATT_PER_LAT;

    if frac == 0 {
        format!("{}", whole)
    } else {
        let frac_str = format!("{:08}", frac);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, trimmed)
    }
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

/// Simple datetime formatter (no external dependency)
fn chrono_format(unix_secs: u64) -> String {
    // Calculate datetime components
    let secs_per_day = 86400u64;
    let secs_per_hour = 3600u64;
    let secs_per_minute = 60u64;

    let days_since_epoch = unix_secs / secs_per_day;
    let remaining_secs = unix_secs % secs_per_day;

    let hours = remaining_secs / secs_per_hour;
    let remaining = remaining_secs % secs_per_hour;
    let minutes = remaining / secs_per_minute;
    let seconds = remaining % secs_per_minute;

    // Calculate year/month/day (simplified, doesn't handle leap seconds perfectly)
    let mut year = 1970;
    let mut remaining_days = days_since_epoch as i64;

    loop {
        let days_in_year = if is_leap_year(year) { 366 } else { 365 };
        if remaining_days < days_in_year {
            break;
        }
        remaining_days -= days_in_year;
        year += 1;
    }

    let mut month = 1;
    let days_in_months = if is_leap_year(year) {
        [31, 29, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    } else {
        [31, 28, 31, 30, 31, 30, 31, 31, 30, 31, 30, 31]
    };

    for days in days_in_months.iter() {
        if remaining_days < *days as i64 {
            break;
        }
        remaining_days -= *days as i64;
        month += 1;
    }

    let day = remaining_days + 1;

    format!(
        "{:04}-{:02}-{:02} {:02}:{:02}:{:02} UTC",
        year, month, day, hours, minutes, seconds
    )
}

fn is_leap_year(year: i32) -> bool {
    (year % 4 == 0 && year % 100 != 0) || (year % 400 == 0)
}
