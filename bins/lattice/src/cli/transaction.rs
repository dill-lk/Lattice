//! Transaction command handlers

use anyhow::{anyhow, Result};
use colored::Colorize;
use lattice_core::{Address, Transaction};
use lattice_wallet::TransactionBuilder;

use crate::cli::rpc_client::RpcClient;
use crate::cli::wallet::load_wallet;

/// Send tokens to a recipient directly
pub async fn send_transaction(
    wallet_path: &str,
    to: &str,
    amount: u128,
    fee: u128,
    gas_limit: Option<u64>,
    rpc_url: &str,
) -> Result<()> {
    let recipient = Address::from_base58(to).map_err(|_| anyhow!("Invalid recipient address"))?;
    let mut account = load_wallet(wallet_path)?;
    let sender_address = account.address().clone();

    println!();
    println!("  {}", "Send Transaction".bold().cyan());
    println!("  {}", "─".repeat(50).dimmed());
    println!();
    println!("  {}  {}", "To".dimmed(), to.white());
    println!("  {}  {}", "Amount".dimmed(), format!("{} LAT", format_amount(amount)).green());
    println!("  {}  {}", "Fee".dimmed(), format!("{} LAT", format_amount(fee)).dimmed());
    println!("  {}  {}", "From".dimmed(), sender_address.to_string().dimmed());

    let client = RpcClient::new(rpc_url);

    let nonce = match client.get_transaction_count(&sender_address.to_base58()).await {
        Ok(n) => n,
        Err(_) => {
            println!("  {} Could not fetch nonce, using 0", "!".yellow());
            0
        }
    };

    account.set_nonce(nonce);

    let mut builder = TransactionBuilder::transfer()
        .to(recipient)
        .amount(amount)
        .fee(fee)
        .nonce(account.next_nonce());

    if let Some(gas) = gas_limit {
        builder = builder.gas_limit(gas);
    }

    let tx = builder.build(&mut account)?;
    let tx_bytes = borsh::to_vec(&tx)?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));

    println!();
    println!("  {} Signing...", "●".cyan());

    match client.send_raw_transaction(&tx_hex).await {
        Ok(hash) => {
            println!("  {} Broadcasting...", "●".cyan());
            println!();
            println!("  {}", "─".repeat(50).dimmed());
            println!("  {} Transaction sent", "✓".green().bold());
            println!();
            println!("  {}  {}", "Hash".dimmed(), hash.white());
            println!();
        }
        Err(e) => {
            println!();
            println!("  {} Failed: {}", "✗".red(), e);
            println!("  {} {}", "Node".dimmed(), rpc_url.dimmed());
            println!();
        }
    }

    Ok(())
}

/// Sign a transaction offline and output the raw hex
pub fn sign_transaction(
    wallet_path: &str,
    to: &str,
    amount: u128,
    fee: u128,
    nonce: u64,
    gas_limit: Option<u64>,
    data_hex: Option<&str>,
) -> Result<()> {
    let recipient = Address::from_base58(to).map_err(|_| anyhow!("Invalid recipient address"))?;
    let mut account = load_wallet(wallet_path)?;
    let sender_address = account.address().clone();

    let data = match data_hex {
        Some(hex_str) => {
            let clean = hex_str.strip_prefix("0x").unwrap_or(hex_str);
            hex::decode(clean).map_err(|_| anyhow!("Invalid data hex format"))?
        }
        None => Vec::new(),
    };

    println!();
    println!("  {}", "Sign Transaction (Offline)".bold().cyan());
    println!("  {}", "─".repeat(50).dimmed());
    println!();
    println!("  {}  {}", "To".dimmed(), to.white());
    println!("  {}  {} LAT", "Amount".dimmed(), format_amount(amount).green());
    println!("  {}   {} LAT", "Fee".dimmed(), format_amount(fee));
    println!("  {}  {}", "From".dimmed(), sender_address.to_string().dimmed());
    println!("  {} {}", "Nonce".dimmed(), nonce);

    account.set_nonce(nonce);

    let mut builder = TransactionBuilder::transfer()
        .to(recipient)
        .amount(amount)
        .fee(fee)
        .data(data)
        .nonce(nonce);

    if let Some(gas) = gas_limit {
        builder = builder.gas_limit(gas);
    }

    let tx = builder.build(&mut account)?;
    let tx_bytes = borsh::to_vec(&tx)?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));

    println!();
    println!("  {}", "─".repeat(50).dimmed());
    println!("  {} Signed raw transaction hex:", "✓".green().bold());
    println!();
    println!("  {}", tx_hex.white());
    println!();

    Ok(())
}

/// Broadcast a raw transaction hex to the network
pub async fn broadcast_transaction(raw_hex: &str, rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);
    let hex_clean = if raw_hex.starts_with("0x") {
        raw_hex.to_string()
    } else {
        format!("0x{}", raw_hex)
    };

    println!();
    println!("  {}", "Broadcast Raw Transaction".bold().cyan());
    println!("  {}", "─".repeat(50).dimmed());
    println!();
    println!("  {} Broadcasting to node: {}...", "●".cyan(), rpc_url);

    match client.send_raw_transaction(&hex_clean).await {
        Ok(hash) => {
            println!();
            println!("  {}", "─".repeat(50).dimmed());
            println!("  {} Transaction broadcasted successfully", "✓".green().bold());
            println!();
            println!("  {}  {}", "Hash".dimmed(), hash.white());
            println!();
        }
        Err(e) => {
            println!();
            println!("  {} Failed: {}", "✗".red(), e);
            println!();
        }
    }
    Ok(())
}

/// Deploy a smart contract
pub async fn deploy_contract(
    wallet_path: &str,
    wasm_path: &str,
    fee: u128,
    gas_limit: Option<u64>,
    rpc_url: &str,
) -> Result<()> {
    let wasm_bytes = std::fs::read(wasm_path).map_err(|e| anyhow!("Failed to read WASM file: {}", e))?;
    
    let mut account = load_wallet(wallet_path)?;
    let sender_address = account.address().clone();

    println!();
    println!("  {}", "Deploy Smart Contract".bold().cyan());
    println!("  {}", "─".repeat(50).dimmed());
    println!();
    println!("  {}  {}", "WASM File".dimmed(), wasm_path.white());
    println!("  {}  {} bytes", "Size".dimmed(), wasm_bytes.len());
    println!("  {}   {} LAT", "Fee".dimmed(), format_amount(fee).dimmed());
    println!("  {}  {}", "Sender".dimmed(), sender_address.to_string().dimmed());

    let client = RpcClient::new(rpc_url);

    let nonce = match client.get_transaction_count(&sender_address.to_base58()).await {
        Ok(n) => n,
        Err(_) => 0,
    };
    account.set_nonce(nonce);

    let mut builder = TransactionBuilder::deploy()
        .amount(0)
        .fee(fee)
        .data(wasm_bytes)
        .nonce(account.next_nonce());

    if let Some(gas) = gas_limit {
        builder = builder.gas_limit(gas);
    }

    let tx = builder.build(&mut account)?;
    let tx_bytes = borsh::to_vec(&tx)?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));

    println!();
    println!("  {} Signing & Broadcasting...", "●".cyan());

    match client.send_raw_transaction(&tx_hex).await {
        Ok(hash) => {
            println!();
            println!("  {}", "─".repeat(50).dimmed());
            println!("  {} Deployment transaction sent", "✓".green().bold());
            println!();
            println!("  {}  {}", "Tx Hash".dimmed(), hash.white());
            println!("  {}  To check contract address, run status command on this hash", "ℹ".cyan());
            println!();
        }
        Err(e) => {
            println!();
            println!("  {} Failed: {}", "✗".red(), e);
            println!();
        }
    }
    Ok(())
}

/// Call a smart contract method
pub async fn call_contract(
    wallet_path: &str,
    contract_addr: &str,
    method: &str,
    args_hex: Option<&str>,
    amount: u128,
    fee: u128,
    gas_limit: Option<u64>,
    rpc_url: &str,
) -> Result<()> {
    let contract = Address::from_base58(contract_addr).map_err(|_| anyhow!("Invalid contract address"))?;
    let mut account = load_wallet(wallet_path)?;
    let sender_address = account.address().clone();

    let mut payload = Vec::new();
    let method_bytes = method.as_bytes();
    payload.push(method_bytes.len() as u8);
    payload.extend_from_slice(method_bytes);

    if let Some(args) = args_hex {
        let args_clean = args.strip_prefix("0x").unwrap_or(args);
        let args_bytes = hex::decode(args_clean).map_err(|_| anyhow!("Invalid args hex encoding"))?;
        payload.extend(args_bytes);
    }

    println!();
    println!("  {}", "Call Smart Contract".bold().cyan());
    println!("  {}", "─".repeat(50).dimmed());
    println!();
    println!("  {}  {}", "Contract".dimmed(), contract_addr.white());
    println!("  {}    {}", "Method".dimmed(), method.yellow());
    println!("  {}    {} LAT", "Value".dimmed(), format_amount(amount).green());
    println!("  {}      {} LAT", "Fee".dimmed(), format_amount(fee).dimmed());

    let client = RpcClient::new(rpc_url);

    let nonce = match client.get_transaction_count(&sender_address.to_base58()).await {
        Ok(n) => n,
        Err(_) => 0,
    };
    account.set_nonce(nonce);

    let mut builder = TransactionBuilder::call()
        .to(contract)
        .amount(amount)
        .fee(fee)
        .data(payload)
        .nonce(account.next_nonce());

    if let Some(gas) = gas_limit {
        builder = builder.gas_limit(gas);
    }

    let tx = builder.build(&mut account)?;
    let tx_bytes = borsh::to_vec(&tx)?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));

    println!();
    println!("  {} Broadcasting contract call...", "●".cyan());

    match client.send_raw_transaction(&tx_hex).await {
        Ok(hash) => {
            println!();
            println!("  {}", "─".repeat(50).dimmed());
            println!("  {} Transaction sent", "✓".green().bold());
            println!();
            println!("  {}  {}", "Tx Hash".dimmed(), hash.white());
            println!();
        }
        Err(e) => {
            println!();
            println!("  {} Failed: {}", "✗".red(), e);
            println!();
        }
    }
    Ok(())
}

/// Check transaction status
pub async fn check_status(hash: &str, rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    let hash = if hash.starts_with("0x") {
        hash.to_string()
    } else {
        format!("0x{}", hash)
    };

    match client.get_transaction(&hash).await {
        Ok(Some(tx)) => {
            println!();
            println!("  {}", "Transaction".bold().cyan());
            println!("  {}", "─".repeat(50).dimmed());
            println!("  {}  {}", "Hash".dimmed(), hash.white());

            if let Some(block_hash) = tx.get("blockHash") {
                if !block_hash.is_null() {
                    println!("  {}  {}", "Status".dimmed(), "Confirmed".green());
                    if let Some(bn) = tx.get("blockNumber") {
                        println!("  {}  #{}", "Block".dimmed(), bn.as_str().unwrap_or("?"));
                    }
                } else {
                    println!("  {}  {}", "Status".dimmed(), "Pending".yellow());
                }
            } else {
                println!("  {}  {}", "Status".dimmed(), "Pending".yellow());
            }

            println!("  {}  {}", "From".dimmed(), tx.get("from").and_then(|v| v.as_str()).unwrap_or("?"));
            println!("  {}  {}", "To".dimmed(), tx.get("to").and_then(|v| v.as_str()).unwrap_or("?"));

            if let Some(value) = tx.get("value").and_then(|v| v.as_str()) {
                if let Ok(amount) = parse_hex_u128(value) {
                    println!("  {}  {} LAT", "Value".dimmed(), format_amount(amount).green());
                }
            }

            if let Ok(Some(receipt)) = client.get_transaction_receipt(&hash).await {
                if let Some(status) = receipt.get("status").and_then(|v| v.as_str()) {
                    let exec_status = if status == "0x1" {
                        "Success".green()
                    } else {
                        "Failed".red()
                    };
                    println!("  {}  {}", "Exec".dimmed(), exec_status);
                }
            }
            println!();
        }
        Ok(None) => {
            println!();
            println!("  {} Transaction not found", "!".yellow());
            println!("  {}  {}", "Hash".dimmed(), hash.dimmed());
            println!();
        }
        Err(e) => {
            println!();
            println!("  {} Query failed: {}", "✗".red(), e);
            println!();
        }
    }

    Ok(())
}

/// Decode a raw transaction
pub fn decode_transaction(raw_tx: &str) -> Result<()> {
    let hex_str = raw_tx.strip_prefix("0x").unwrap_or(raw_tx);
    let tx_bytes = hex::decode(hex_str).map_err(|_| anyhow!("Invalid hex encoding"))?;

    let tx: Transaction =
        borsh::from_slice(&tx_bytes).map_err(|e| anyhow!("Failed to decode transaction: {}", e))?;

    println!();
    println!("  {}", "Decoded Transaction".bold().cyan());
    println!("  {}", "─".repeat(50).dimmed());
    println!("  {}  0x{}", "Hash".dimmed(), hex::encode(tx.hash()));
    println!("  {}  {:?}", "Kind".dimmed(), tx.kind);
    println!("  {}  {}", "From".dimmed(), tx.from);
    println!("  {}  {}", "To".dimmed(), tx.to);
    println!("  {}  {} LAT", "Amount".dimmed(), format_amount(tx.amount).green());
    println!("  {}  {} LAT", "Fee".dimmed(), format_amount(tx.fee));
    println!("  {}  {}", "Nonce".dimmed(), tx.nonce);
    println!("  {}  {}", "Gas".dimmed(), tx.gas_limit);

    if !tx.data.is_empty() {
        println!("  {}  {} bytes", "Data".dimmed(), tx.data.len());
    }

    if tx.is_signed() {
        let sig_status = if tx.verify_signature() {
            "Valid".green()
        } else {
            "Invalid".red()
        };
        println!("  {}  {}", "Sig".dimmed(), sig_status);
    } else {
        println!("  {}  {}", "Sig".dimmed(), "None".dimmed());
    }
    println!();

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

/// Parse hex string to u128
fn parse_hex_u128(s: &str) -> Result<u128> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    u128::from_str_radix(s, 16).map_err(|e| anyhow!("Invalid hex number: {}", e))
}
