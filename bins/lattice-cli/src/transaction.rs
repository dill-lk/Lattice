//! Transaction command handlers

use anyhow::{anyhow, bail, Result};
use lattice_core::{Address, Transaction};
use lattice_wallet::TransactionBuilder;

use crate::rpc_client::RpcClient;
use crate::wallet::load_wallet;

/// Send tokens to a recipient
pub async fn send_transaction(
    wallet_path: &str,
    to: &str,
    amount: u128,
    fee: u128,
    gas_limit: Option<u64>,
    rpc_url: &str,
) -> Result<()> {
    // Parse recipient address
    let recipient = Address::from_base58(to).map_err(|_| anyhow!("Invalid recipient address"))?;

    // Load wallet
    let mut account = load_wallet(wallet_path)?;
    let sender_address = account.address().clone();

    println!("Sending {} LAT to {}", format_amount(amount), to);
    println!("From: {}", sender_address);
    println!("Fee: {} LAT", format_amount(fee));

    // Create RPC client
    let client = RpcClient::new(rpc_url);

    // Get current nonce from network
    let nonce = match client.get_transaction_count(&sender_address.to_base58()).await {
        Ok(n) => n,
        Err(_) => {
            println!("Warning: Could not fetch nonce from network, using 0");
            0
        }
    };

    account.set_nonce(nonce);

    // Build transaction
    let mut builder = TransactionBuilder::transfer()
        .to(recipient)
        .amount(amount)
        .fee(fee)
        .nonce(account.next_nonce());

    if let Some(gas) = gas_limit {
        builder = builder.gas_limit(gas);
    }

    let tx = builder.build(&mut account)?;

    // Serialize transaction
    let tx_bytes = borsh::to_vec(&tx)?;
    let tx_hex = format!("0x{}", hex::encode(&tx_bytes));

    println!("Transaction hash: 0x{}", hex::encode(tx.hash()));

    // Send to network
    match client.send_raw_transaction(&tx_hex).await {
        Ok(hash) => {
            println!("✓ Transaction submitted: {}", hash);
            println!();
            println!("Use 'lattice-cli tx status {}' to check status", hash);
        }
        Err(e) => {
            eprintln!("✗ Failed to submit transaction: {}", e);
            eprintln!();
            eprintln!("Make sure the Lattice node is running at: {}", rpc_url);
        }
    }

    Ok(())
}

/// Check transaction status
pub async fn check_status(hash: &str, rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    // Normalize hash format
    let hash = if hash.starts_with("0x") {
        hash.to_string()
    } else {
        format!("0x{}", hash)
    };

    // Try to get transaction
    match client.get_transaction(&hash).await {
        Ok(Some(tx)) => {
            println!("Transaction: {}", hash);
            println!();

            if let Some(block_hash) = tx.get("blockHash") {
                if !block_hash.is_null() {
                    println!("Status: ✓ Confirmed");
                    if let Some(bn) = tx.get("blockNumber") {
                        println!("Block: {}", bn.as_str().unwrap_or("unknown"));
                    }
                } else {
                    println!("Status: ⏳ Pending");
                }
            } else {
                println!("Status: ⏳ Pending");
            }

            println!();
            println!("From: {}", tx.get("from").and_then(|v| v.as_str()).unwrap_or("?"));
            println!("To: {}", tx.get("to").and_then(|v| v.as_str()).unwrap_or("?"));

            if let Some(value) = tx.get("value").and_then(|v| v.as_str()) {
                if let Ok(amount) = parse_hex_u128(value) {
                    println!("Value: {} LAT", format_amount(amount));
                }
            }

            if let Some(gas) = tx.get("gas").and_then(|v| v.as_str()) {
                println!("Gas Limit: {}", gas);
            }

            // Try to get receipt for more details
            if let Ok(Some(receipt)) = client.get_transaction_receipt(&hash).await {
                println!();
                if let Some(status) = receipt.get("status").and_then(|v| v.as_str()) {
                    if status == "0x1" {
                        println!("Execution: ✓ Success");
                    } else {
                        println!("Execution: ✗ Failed");
                    }
                }
                if let Some(gas_used) = receipt.get("gasUsed").and_then(|v| v.as_str()) {
                    println!("Gas Used: {}", gas_used);
                }
            }
        }
        Ok(None) => {
            println!("Transaction not found: {}", hash);
            println!();
            println!("The transaction may not exist or hasn't been broadcast yet.");
        }
        Err(e) => {
            eprintln!("Failed to query transaction: {}", e);
            eprintln!();
            eprintln!("Make sure the Lattice node is running at: {}", rpc_url);
        }
    }

    Ok(())
}

/// Decode a raw transaction
pub fn decode_transaction(raw_tx: &str) -> Result<()> {
    // Remove 0x prefix if present
    let hex_str = raw_tx.strip_prefix("0x").unwrap_or(raw_tx);

    // Decode hex
    let tx_bytes = hex::decode(hex_str).map_err(|_| anyhow!("Invalid hex encoding"))?;

    // Deserialize transaction
    let tx: Transaction =
        borsh::from_slice(&tx_bytes).map_err(|e| anyhow!("Failed to decode transaction: {}", e))?;

    // Display transaction details
    println!("Transaction Details");
    println!("==================");
    println!();
    println!("Hash: 0x{}", hex::encode(tx.hash()));
    println!("Kind: {:?}", tx.kind);
    println!("From: {}", tx.from);
    println!("To: {}", tx.to);
    println!("Amount: {} LAT", format_amount(tx.amount));
    println!("Fee: {} LAT", format_amount(tx.fee));
    println!("Nonce: {}", tx.nonce);
    println!("Gas Limit: {}", tx.gas_limit);
    println!("Chain ID: {}", tx.chain_id);

    if !tx.data.is_empty() {
        println!("Data: 0x{}", hex::encode(&tx.data));
        println!("Data Size: {} bytes", tx.data.len());
    }

    println!();
    if tx.is_signed() {
        println!("Signature: Present ({} bytes)", tx.signature.len());
        println!("Public Key: Present ({} bytes)", tx.public_key.len());

        if tx.verify_signature() {
            println!("Signature Valid: ✓ Yes");
        } else {
            println!("Signature Valid: ✗ No (verification failed)");
        }
    } else {
        println!("Signature: Not signed");
    }

    Ok(())
}

/// Format amount in LAT (18 decimals)
fn format_amount(amount: u128) -> String {
    let whole = amount / 1_000_000_000_000_000_000u128;
    let frac = amount % 1_000_000_000_000_000_000u128;

    if frac == 0 {
        format!("{}", whole)
    } else {
        // Trim trailing zeros
        let frac_str = format!("{:018}", frac);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{}", whole, trimmed)
    }
}

/// Parse hex string to u128
fn parse_hex_u128(s: &str) -> Result<u128> {
    let s = s.strip_prefix("0x").unwrap_or(s);
    u128::from_str_radix(s, 16).map_err(|e| anyhow!("Invalid hex number: {}", e))
}
