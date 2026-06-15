//! Transaction and contract command handlers.

use anyhow::{anyhow, Result};
use colored::Colorize;
use lattice_core::{Address, Transaction};
use lattice_wallet::TransactionBuilder;

use crate::cli::formatter;
use crate::cli::output;
use crate::cli::rpc_client::RpcClient;
use crate::cli::wallet::load_wallet;

/// Send tokens to a recipient directly.
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

    formatter::title("Send Transaction");
    formatter::divider();
    formatter::key_value("From", &sender_address.to_string());
    formatter::key_value("To", to);
    formatter::key_value("Amount", &formatter::format_amount(amount));
    formatter::key_value("Fee", &formatter::format_amount(fee));
    formatter::key_value("RPC", rpc_url);

    let client = RpcClient::new(rpc_url);
    let nonce = match client
        .get_transaction_count(&sender_address.to_base58())
        .await
    {
        Ok(value) => value,
        Err(_) => {
            formatter::warning("Could not fetch nonce from RPC. Falling back to 0.");
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
        formatter::key_value("Gas Limit", &gas.to_string());
    }

    let tx = builder.build(&mut account)?;
    let tx_bytes = borsh::to_vec(&tx)?;
    let tx_hex = format!("0x{}", hex::encode(tx_bytes));

    formatter::info("Signing transaction…");
    match client.send_raw_transaction(&tx_hex).await {
        Ok(hash) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": true,
                    "action": "tx_send",
                    "from": sender_address.to_string(),
                    "to": to,
                    "amount_latt": amount,
                    "fee_latt": fee,
                    "hash": hash,
                    "rpc": rpc_url,
                }));
            }
            formatter::success("Transaction submitted successfully.");
            formatter::key_value("Transaction Hash", &hash.white().to_string());
        }
        Err(e) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "tx_send",
                    "error": e.to_string(),
                    "rpc": rpc_url,
                }));
            }
            formatter::error(&format!("Broadcast failed: {e}"));
            formatter::note("Verify the node is reachable and the transaction is valid.");
        }
    }

    println!();
    Ok(())
}

/// Sign a transaction offline and output the raw hex.
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
        Some(hex_str) => hex::decode(hex_str.trim_start_matches("0x"))
            .map_err(|_| anyhow!("Invalid data hex format"))?,
        None => Vec::new(),
    };

    formatter::title("Offline Transaction Signing");
    formatter::divider();
    formatter::key_value("From", &sender_address.to_string());
    formatter::key_value("To", to);
    formatter::key_value("Amount", &formatter::format_amount(amount));
    formatter::key_value("Fee", &formatter::format_amount(fee));
    formatter::key_value("Nonce", &nonce.to_string());
    if !data.is_empty() {
        formatter::key_value("Payload", &format!("{} bytes", data.len()));
    }

    account.set_nonce(nonce);

    let mut builder = TransactionBuilder::transfer()
        .to(recipient)
        .amount(amount)
        .fee(fee)
        .data(data)
        .nonce(nonce);

    if let Some(gas) = gas_limit {
        builder = builder.gas_limit(gas);
        formatter::key_value("Gas Limit", &gas.to_string());
    }

    let tx = builder.build(&mut account)?;
    let tx_bytes = borsh::to_vec(&tx)?;
    let tx_hex = format!("0x{}", hex::encode(tx_bytes));

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "tx_sign",
            "from": sender_address.to_string(),
            "to": to,
            "amount_latt": amount,
            "fee_latt": fee,
            "nonce": nonce,
            "raw": tx_hex,
        }));
    }

    formatter::success("Transaction signed.");
    formatter::subheader("Raw Transaction Hex");
    println!("  {}", tx_hex.white());
    println!();
    Ok(())
}

/// Broadcast a raw transaction hex to the network.
pub async fn broadcast_transaction(raw_hex: &str, rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);
    let hex_clean = if raw_hex.starts_with("0x") {
        raw_hex.to_string()
    } else {
        format!("0x{raw_hex}")
    };

    formatter::title("Broadcast Raw Transaction");
    formatter::divider();
    formatter::key_value("RPC", rpc_url);
    formatter::key_value(
        "Payload Size",
        &format!("{} bytes", hex_clean.trim_start_matches("0x").len() / 2),
    );
    formatter::info("Submitting raw transaction to node…");

    match client.send_raw_transaction(&hex_clean).await {
        Ok(hash) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": true,
                    "action": "tx_broadcast",
                    "hash": hash,
                    "rpc": rpc_url,
                }));
            }
            formatter::success("Transaction broadcasted successfully.");
            formatter::key_value("Transaction Hash", &hash);
        }
        Err(e) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "tx_broadcast",
                    "error": e.to_string(),
                    "rpc": rpc_url,
                }));
            }
            formatter::error(&format!("Broadcast failed: {e}"));
        }
    }

    println!();
    Ok(())
}

/// Deploy a smart contract.
pub async fn deploy_contract(
    wallet_path: &str,
    wasm_path: &str,
    fee: u128,
    gas_limit: Option<u64>,
    rpc_url: &str,
) -> Result<()> {
    let wasm_bytes =
        std::fs::read(wasm_path).map_err(|e| anyhow!("Failed to read WASM file: {e}"))?;

    let mut account = load_wallet(wallet_path)?;
    let sender_address = account.address().clone();

    formatter::title("Deploy Smart Contract");
    formatter::divider();
    formatter::key_value("Sender", &sender_address.to_string());
    formatter::key_value("WASM File", wasm_path);
    formatter::key_value("Size", &format!("{} bytes", wasm_bytes.len()));
    formatter::key_value("Fee", &formatter::format_amount(fee));
    formatter::key_value("RPC", rpc_url);

    let client = RpcClient::new(rpc_url);
    let nonce = client
        .get_transaction_count(&sender_address.to_base58())
        .await
        .unwrap_or(0);
    account.set_nonce(nonce);

    let mut builder = TransactionBuilder::deploy()
        .amount(0)
        .fee(fee)
        .data(wasm_bytes)
        .nonce(account.next_nonce());

    if let Some(gas) = gas_limit {
        builder = builder.gas_limit(gas);
        formatter::key_value("Gas Limit", &gas.to_string());
    }

    let tx = builder.build(&mut account)?;
    let tx_bytes = borsh::to_vec(&tx)?;
    let tx_hex = format!("0x{}", hex::encode(tx_bytes));

    formatter::info("Submitting deployment transaction…");
    match client.send_raw_transaction(&tx_hex).await {
        Ok(hash) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": true,
                    "action": "contract_deploy",
                    "sender": sender_address.to_string(),
                    "wasm": wasm_path,
                    "fee_latt": fee,
                    "hash": hash,
                }));
            }
            formatter::success("Deployment transaction submitted.");
            formatter::key_value("Transaction Hash", &hash);
            formatter::note(
                "Use `lattice tx status <hash>` to track confirmation and receipt status.",
            );
        }
        Err(e) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "contract_deploy",
                    "error": e.to_string(),
                }));
            }
            formatter::error(&format!("Deployment failed: {e}"));
        }
    }

    println!();
    Ok(())
}

/// Options for calling a smart contract method.
pub struct CallContractOptions<'a> {
    pub wallet_path: &'a str,
    pub contract_addr: &'a str,
    pub method: &'a str,
    pub args_hex: Option<&'a str>,
    pub amount: u128,
    pub fee: u128,
    pub gas_limit: Option<u64>,
    pub rpc_url: &'a str,
}

/// Call a smart contract method.
pub async fn call_contract(options: CallContractOptions<'_>) -> Result<()> {
    let CallContractOptions {
        wallet_path,
        contract_addr,
        method,
        args_hex,
        amount,
        fee,
        gas_limit,
        rpc_url,
    } = options;

    let contract =
        Address::from_base58(contract_addr).map_err(|_| anyhow!("Invalid contract address"))?;
    let mut account = load_wallet(wallet_path)?;
    let sender_address = account.address().clone();

    let mut payload = Vec::new();
    payload.push(method.len() as u8);
    payload.extend_from_slice(method.as_bytes());

    if let Some(args) = args_hex {
        let args_bytes = hex::decode(args.trim_start_matches("0x"))
            .map_err(|_| anyhow!("Invalid args hex encoding"))?;
        payload.extend(args_bytes);
    }

    formatter::title("Call Smart Contract");
    formatter::divider();
    formatter::key_value("Caller", &sender_address.to_string());
    formatter::key_value("Contract", contract_addr);
    formatter::key_value("Method", method);
    formatter::key_value("Value", &formatter::format_amount(amount));
    formatter::key_value("Fee", &formatter::format_amount(fee));
    formatter::key_value("RPC", rpc_url);
    formatter::key_value("Payload", &format!("{} bytes", payload.len()));

    let client = RpcClient::new(rpc_url);
    let nonce = client
        .get_transaction_count(&sender_address.to_base58())
        .await
        .unwrap_or(0);
    account.set_nonce(nonce);

    let mut builder = TransactionBuilder::call()
        .to(contract)
        .amount(amount)
        .fee(fee)
        .data(payload)
        .nonce(account.next_nonce());

    if let Some(gas) = gas_limit {
        builder = builder.gas_limit(gas);
        formatter::key_value("Gas Limit", &gas.to_string());
    }

    let tx = builder.build(&mut account)?;
    let tx_bytes = borsh::to_vec(&tx)?;
    let tx_hex = format!("0x{}", hex::encode(tx_bytes));

    formatter::info("Submitting contract call…");
    match client.send_raw_transaction(&tx_hex).await {
        Ok(hash) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": true,
                    "action": "contract_call",
                    "caller": sender_address.to_string(),
                    "contract": contract_addr,
                    "method": method,
                    "amount_latt": amount,
                    "fee_latt": fee,
                    "hash": hash,
                }));
            }
            formatter::success("Contract call submitted.");
            formatter::key_value("Transaction Hash", &hash);
        }
        Err(e) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "contract_call",
                    "error": e.to_string(),
                }));
            }
            formatter::error(&format!("Contract call failed: {e}"));
        }
    }

    println!();
    Ok(())
}

/// Check transaction status.
pub async fn check_status(hash: &str, rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);
    let hash = if hash.starts_with("0x") {
        hash.to_string()
    } else {
        format!("0x{hash}")
    };

    match client.get_transaction(&hash).await {
        Ok(Some(tx)) => {
            let confirmed = tx.get("blockHash").is_some_and(|v| !v.is_null());
            let receipt = client.get_transaction_receipt(&hash).await.ok().flatten();

            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": true,
                    "action": "tx_status",
                    "hash": hash,
                    "confirmed": confirmed,
                    "transaction": tx,
                    "receipt": receipt,
                }));
            }

            formatter::title("Transaction Status");
            formatter::divider();
            formatter::key_value("Hash", &hash);

            if confirmed {
                formatter::key_value_colored("State", "CONFIRMED".green().bold());
                if let Some(number) = tx.get("blockNumber").and_then(|v| v.as_str()) {
                    formatter::key_value("Block", number);
                }
            } else {
                formatter::key_value_colored("State", "PENDING".yellow().bold());
            }

            formatter::key_value(
                "From",
                tx.get("from").and_then(|v| v.as_str()).unwrap_or("?"),
            );
            formatter::key_value("To", tx.get("to").and_then(|v| v.as_str()).unwrap_or("?"));

            if let Some(value) = tx.get("value").and_then(|v| v.as_str()) {
                if let Ok(amount) = parse_hex_u128(value) {
                    formatter::key_value("Value", &formatter::format_amount(amount));
                }
            }

            if let Some(receipt) = receipt {
                if let Some(status) = receipt.get("status").and_then(|v| v.as_str()) {
                    let rendered = if status == "0x1" {
                        "SUCCESS".green().bold()
                    } else {
                        "FAILED".red().bold()
                    };
                    formatter::key_value_colored("Execution", rendered);
                }
            }
            println!();
        }
        Ok(None) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "tx_status",
                    "hash": hash,
                    "error": "transaction not found",
                }));
            }
            formatter::warning("Transaction not found.");
            formatter::key_value("Hash", &hash);
            println!();
        }
        Err(e) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "tx_status",
                    "hash": hash,
                    "error": e.to_string(),
                }));
            }
            formatter::error(&format!("Transaction lookup failed: {e}"));
            println!();
        }
    }

    Ok(())
}

/// Decode a raw transaction.
pub fn decode_transaction(raw_tx: &str) -> Result<()> {
    let tx_bytes = hex::decode(raw_tx.trim_start_matches("0x"))
        .map_err(|_| anyhow!("Invalid hex encoding"))?;

    let tx: Transaction =
        borsh::from_slice(&tx_bytes).map_err(|e| anyhow!("Failed to decode transaction: {e}"))?;

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "tx_decode",
            "hash": format!("0x{}", hex::encode(tx.hash())),
            "kind": format!("{:?}", tx.kind),
            "from": tx.from.to_string(),
            "to": tx.to.to_string(),
            "amount_latt": tx.amount,
            "fee_latt": tx.fee,
            "nonce": tx.nonce,
            "gas": tx.gas_limit,
            "data_bytes": tx.data.len(),
            "signature_present": tx.is_signed(),
            "signature_valid": tx.verify_signature(),
        }));
    }

    formatter::title("Decoded Transaction");
    formatter::divider();
    formatter::key_value("Hash", &format!("0x{}", hex::encode(tx.hash())));
    formatter::key_value("Kind", &format!("{:?}", tx.kind));
    formatter::key_value("From", &tx.from.to_string());
    formatter::key_value("To", &tx.to.to_string());
    formatter::key_value("Amount", &formatter::format_amount(tx.amount));
    formatter::key_value("Fee", &formatter::format_amount(tx.fee));
    formatter::key_value("Nonce", &tx.nonce.to_string());
    formatter::key_value("Gas", &tx.gas_limit.to_string());

    if !tx.data.is_empty() {
        formatter::key_value("Data", &format!("{} bytes", tx.data.len()));
    }

    if tx.is_signed() {
        if tx.verify_signature() {
            formatter::key_value_colored("Signature", "VALID".green().bold());
        } else {
            formatter::key_value_colored("Signature", "INVALID".red().bold());
        }
    } else {
        formatter::key_value("Signature", "not present");
    }

    println!();
    Ok(())
}

fn parse_hex_u128(s: &str) -> Result<u128> {
    u128::from_str_radix(s.trim_start_matches("0x"), 16)
        .map_err(|e| anyhow!("Invalid hex number: {e}"))
}
