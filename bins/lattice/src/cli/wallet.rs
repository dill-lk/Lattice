//! Wallet command handlers.

use anyhow::{anyhow, bail, Result};
use colored::Colorize;
use dialoguer::{Confirm, Password};
use lattice_core::Address;
use lattice_wallet::{Keystore, WalletAccount};
use std::path::Path;

use crate::cli::formatter;
use crate::cli::output;
use crate::cli::rpc_client::RpcClient;

const DEFAULT_WALLET_CONFIG_DIR: &str = ".lattice";
const DEFAULT_WALLET_CONFIG_FILE: &str = "default_wallet";

/// Create a new wallet and save it to an encrypted keystore file.
pub fn create_wallet(output: &str) -> Result<()> {
    let path = Path::new(output);
    if path.exists() {
        bail!("Wallet file already exists: {output}");
    }

    formatter::title("Create Wallet");
    formatter::divider();
    formatter::key_value("Output File", output);
    formatter::info("Generating post-quantum Dilithium account…");

    let account = WalletAccount::generate();
    let address = account.address().clone();

    let password = Password::new()
        .with_prompt("  Enter password")
        .with_confirmation("  Confirm password", "  Passwords do not match")
        .interact()?;

    if password.len() < 8 {
        bail!("Password must be at least 8 characters");
    }

    formatter::info("Encrypting keystore with Argon2id + AES-256-GCM…");
    let keystore = Keystore::encrypt(&account, &password)?;
    keystore.save_to_file(path)?;

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "wallet_create",
            "address": address.to_string(),
            "keystore": output,
        }));
    }

    formatter::success("Wallet created successfully.");
    formatter::key_value("Address", &address.to_string());
    formatter::key_value("Keystore", output);
    formatter::note("Remember your password. It cannot be recovered.");
    println!();
    Ok(())
}

pub fn get_default_wallet_path() -> String {
    default_wallet_config_path()
        .and_then(|path| std::fs::read_to_string(path).ok())
        .map(|contents| contents.trim().to_string())
        .filter(|value| !value.is_empty())
        .unwrap_or_else(|| "wallet.json".to_string())
}

pub fn show_default_wallet() -> Result<()> {
    let wallet = get_default_wallet_path();
    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "wallet_default_show",
            "wallet": wallet,
        }));
    }

    formatter::title("Default Wallet");
    formatter::divider();
    formatter::key_value("Wallet", &wallet);
    println!();
    Ok(())
}

pub fn set_default_wallet(wallet_path: &str) -> Result<()> {
    let path = Path::new(wallet_path);
    if !path.exists() {
        bail!("Wallet file not found: {wallet_path}");
    }

    let config_path = default_wallet_config_path()
        .ok_or_else(|| anyhow!("Unable to resolve default wallet config path"))?;
    if let Some(parent) = config_path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(&config_path, wallet_path)?;

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "wallet_default_set",
            "wallet": wallet_path,
            "config": config_path.to_string_lossy(),
        }));
    }

    formatter::title("Default Wallet Updated");
    formatter::divider();
    formatter::key_value("Wallet", wallet_path);
    formatter::key_value("Config", &config_path.to_string_lossy());
    println!();
    Ok(())
}

pub fn rename_wallet(from: &str, to: &str) -> Result<()> {
    let from_path = Path::new(from);
    let to_path = Path::new(to);
    if !from_path.exists() {
        bail!("Wallet file not found: {from}");
    }
    if to_path.exists() {
        bail!("Destination already exists: {to}");
    }

    std::fs::rename(from_path, to_path)?;

    let current_default = get_default_wallet_path();
    if current_default == from {
        if let Some(config_path) = default_wallet_config_path() {
            if let Some(parent) = config_path.parent() {
                let _ = std::fs::create_dir_all(parent);
            }
            let _ = std::fs::write(config_path, to);
        }
    }

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "wallet_rename",
            "from": from,
            "to": to,
        }));
    }

    formatter::title("Wallet Renamed");
    formatter::divider();
    formatter::key_value("From", from);
    formatter::key_value("To", to);
    println!();
    Ok(())
}

/// Import wallet from private key (hex-encoded).
pub fn import_from_private_key(private_key: &str, output: &str) -> Result<()> {
    let path = Path::new(output);
    if path.exists() {
        bail!("Wallet file already exists: {output}");
    }

    let key_hex = private_key.trim_start_matches("0x");
    let secret_bytes = hex::decode(key_hex).map_err(|_| anyhow!("Invalid hex encoding"))?;

    formatter::title("Import Key Material");
    formatter::divider();
    formatter::key_value("Output", output);
    formatter::note("For Dilithium accounts, safe import requires combined key material containing both public and secret bytes.");

    let account = WalletAccount::from_secret_key(&secret_bytes)
        .map_err(|e| anyhow!("Safe key import failed: {}", e))?;
    let address = account.address().clone();

    let password = Password::new()
        .with_prompt("  Enter password for imported wallet")
        .with_confirmation("  Confirm password", "  Passwords do not match")
        .interact()?;

    if password.len() < 8 {
        bail!("Password must be at least 8 characters");
    }

    let keystore = Keystore::encrypt(&account, &password)?;
    keystore.save_to_file(path)?;

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "wallet_import_key_material",
            "address": address.to_string(),
            "keystore": output,
        }));
    }

    formatter::success("Wallet imported from key material.");
    formatter::key_value("Address", &address.to_string());
    formatter::key_value("Keystore", output);
    formatter::note("Raw secret-only Dilithium imports remain intentionally rejected.");
    println!();
    Ok(())
}

/// Import wallet from mnemonic phrase.
pub fn import_from_mnemonic(_mnemonic: &str, output: &str) -> Result<()> {
    let path = Path::new(output);
    if path.exists() {
        bail!("Wallet file already exists: {output}");
    }

    bail!("Mnemonic import is not yet supported. Use keystore import for now.")
}

/// Import wallet from an existing keystore file.
pub fn import_from_keystore(keystore_path: &str, output: &str) -> Result<()> {
    let input_path = Path::new(keystore_path);
    let output_path = Path::new(output);

    if !input_path.exists() {
        bail!("Keystore file not found: {keystore_path}");
    }
    if output_path.exists() {
        bail!("Output file already exists: {output}");
    }

    formatter::title("Import Keystore");
    formatter::divider();
    formatter::key_value("Source", keystore_path);
    formatter::key_value("Output", output);

    let source_keystore = Keystore::load_from_file(input_path)?;
    let source_password = Password::new()
        .with_prompt("  Enter source keystore password")
        .interact()?;

    formatter::info("Decrypting source keystore…");
    let account = source_keystore.decrypt(&source_password)?;
    let address = account.address().clone();

    let new_password = Password::new()
        .with_prompt("  Enter new password")
        .with_confirmation("  Confirm new password", "  Passwords do not match")
        .interact()?;

    if new_password.len() < 8 {
        bail!("Password must be at least 8 characters");
    }

    formatter::info("Re-encrypting imported wallet…");
    let new_keystore = Keystore::encrypt(&account, &new_password)?;
    new_keystore.save_to_file(output_path)?;

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "wallet_import_keystore",
            "address": address.to_string(),
            "keystore": output,
        }));
    }

    formatter::success("Wallet imported successfully.");
    formatter::key_value("Address", &address.to_string());
    formatter::key_value("Keystore", output);
    println!();
    Ok(())
}

/// Export private key from wallet (dangerous operation).
pub fn export_private_key(wallet_path: &str) -> Result<()> {
    let path = Path::new(wallet_path);
    if !path.exists() {
        bail!("Wallet file not found: {wallet_path}");
    }

    let keystore = Keystore::load_from_file(path)?;

    formatter::title("Export Private Key");
    formatter::divider();
    formatter::warning("Anyone with this key can control the wallet funds.");
    formatter::key_value("Wallet", wallet_path);

    let confirmed = Confirm::new()
        .with_prompt("  Continue with private key export?")
        .default(false)
        .interact()?;
    if !confirmed {
        formatter::note("Export cancelled.");
        println!();
        return Ok(());
    }

    let password = Password::new()
        .with_prompt("  Enter wallet password")
        .interact()?;
    let account = keystore.decrypt(&password)?;

    let double_confirmed = Confirm::new()
        .with_prompt("  Display private key on screen?")
        .default(false)
        .interact()?;
    if !double_confirmed {
        formatter::note("Export cancelled.");
        println!();
        return Ok(());
    }

    let secret_bytes = account.secret_key_bytes();
    formatter::success("Private key export unlocked.");
    formatter::subheader("Secret Key");
    println!("  0x{}", hex::encode(&*secret_bytes).bright_white());
    formatter::warning("Store this securely. Never paste it into untrusted tools.");
    println!();
    Ok(())
}

/// Show wallet address.
pub fn show_address(wallet_path: &str) -> Result<()> {
    let path = Path::new(wallet_path);
    if !path.exists() {
        bail!("Wallet file not found: {wallet_path}");
    }

    let keystore = Keystore::load_from_file(path)?;

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "wallet_address",
            "address": keystore.address(),
            "keystore_id": keystore.id(),
            "file": wallet_path,
        }));
    }

    formatter::title("Wallet Address");
    formatter::divider();
    formatter::key_value("Address", keystore.address());
    formatter::key_value("Keystore ID", keystore.id());
    formatter::key_value("File", wallet_path);
    println!();
    Ok(())
}

/// List all wallets in the current directory.
pub fn list_wallets() -> Result<()> {
    let mut rows = Vec::new();
    let default_wallet = get_default_wallet_path();

    if let Ok(entries) = std::fs::read_dir(".") {
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().is_some_and(|ext| ext == "json") {
                if let Ok(keystore) = Keystore::load_from_file(&path) {
                    let file = path
                        .file_name()
                        .unwrap_or_default()
                        .to_string_lossy()
                        .to_string();
                    rows.push(serde_json::json!({
                        "file": file,
                        "address": keystore.address(),
                        "keystore_id": keystore.id(),
                        "default": default_wallet == path.to_string_lossy() || default_wallet == file,
                    }));
                }
            }
        }
    }

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "wallet_list",
            "count": rows.len(),
            "wallets": rows,
        }));
    }

    formatter::title("Keystores in Current Directory");
    formatter::divider();
    formatter::key_value("Default Wallet", &default_wallet);
    formatter::table_header(&["Default", "File", "Address", "Keystore ID"]);

    for row in &rows {
        formatter::table_row(&[
            if row
                .get("default")
                .and_then(|v| v.as_bool())
                .unwrap_or(false)
            {
                "yes".to_string()
            } else {
                "".to_string()
            },
            row.get("file")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            row.get("address")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
            row.get("keystore_id")
                .and_then(|v| v.as_str())
                .unwrap_or_default()
                .to_string(),
        ]);
    }

    if rows.is_empty() {
        formatter::note("No valid wallet keystores (*.json) were found.");
    }

    println!();
    Ok(())
}

/// Delete a wallet keystore file.
pub fn delete_wallet(wallet_path: &str) -> Result<()> {
    let path = Path::new(wallet_path);
    if !path.exists() {
        bail!("Wallet file not found: {wallet_path}");
    }

    formatter::title("Delete Keystore");
    formatter::divider();
    formatter::key_value("File", wallet_path);
    formatter::warning("Deleting the keystore removes local access to this wallet.");

    let confirmed = Confirm::new()
        .with_prompt("  Permanently delete this keystore file?")
        .default(false)
        .interact()?;

    if confirmed {
        std::fs::remove_file(path)?;
        formatter::success("Keystore deleted.");
    } else {
        formatter::note("Deletion cancelled.");
    }

    println!();
    Ok(())
}

pub fn validate_address(address: &str) -> Result<()> {
    let valid = Address::from_base58(address).is_ok();

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": valid,
            "action": "wallet_validate",
            "address": address,
            "valid": valid,
        }));
    }

    formatter::title("Address Validation");
    formatter::divider();
    formatter::key_value("Address", address);
    if valid {
        formatter::key_value_colored("Valid", "YES".green().bold());
    } else {
        formatter::key_value_colored("Valid", "NO".red().bold());
    }
    println!();
    Ok(())
}

pub async fn show_nonce(address_or_wallet: &str, rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);
    let address = resolve_address_input(address_or_wallet)?;

    match client.get_transaction_count(&address).await {
        Ok(nonce) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": true,
                    "action": "wallet_nonce",
                    "address": address,
                    "nonce": nonce,
                    "rpc": rpc_url,
                }));
            }

            formatter::title("Wallet Nonce");
            formatter::divider();
            formatter::key_value("Address", &address);
            formatter::key_value("Nonce", &nonce.to_string());
            formatter::key_value("RPC", rpc_url);
            println!();
            Ok(())
        }
        Err(e) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "wallet_nonce",
                    "address": address,
                    "rpc": rpc_url,
                    "error": e.to_string(),
                }));
            }
            formatter::title("Wallet Nonce");
            formatter::divider();
            formatter::key_value("Address", &address);
            formatter::error(&format!("RPC request failed: {e}"));
            println!();
            Ok(())
        }
    }
}

/// Query wallet balance via RPC.
pub async fn show_balance(address_or_wallet: &str, rpc_url: &str) -> Result<()> {
    use lattice_core::tokenomics::{
        BLOCKS_PER_MONTH, FOUNDER_IMMEDIATE_AMOUNT, FOUNDER_VESTING_AMOUNT, FOUNDER_WALLET_ADDRESS,
        LATT_PER_LAT, TOKEN_SYMBOL, VESTING_DURATION_MONTHS,
    };

    let client = RpcClient::new(rpc_url);
    let address = resolve_address_input(address_or_wallet)?;

    let is_founder = address == FOUNDER_WALLET_ADDRESS;
    let balance_result = client.get_balance(&address).await;
    let block_height = client.get_block_number().await.unwrap_or(0);

    match balance_result {
        Ok(balance) => {
            let lat = balance as f64 / LATT_PER_LAT as f64;
            let months_elapsed = block_height / BLOCKS_PER_MONTH;
            let months_remaining = VESTING_DURATION_MONTHS.saturating_sub(months_elapsed);
            let vested_amount = if months_elapsed >= VESTING_DURATION_MONTHS {
                FOUNDER_VESTING_AMOUNT
            } else {
                (FOUNDER_VESTING_AMOUNT * months_elapsed as u128) / VESTING_DURATION_MONTHS as u128
            };
            let locked_amount = FOUNDER_VESTING_AMOUNT.saturating_sub(vested_amount);
            let monthly_release = FOUNDER_VESTING_AMOUNT / VESTING_DURATION_MONTHS as u128;

            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": true,
                    "action": "wallet_balance",
                    "address": address,
                    "balance_latt": balance,
                    "balance_lat": lat,
                    "observed_block": block_height,
                    "founder_wallet": is_founder,
                    "vesting": if is_founder { serde_json::json!({
                        "months_elapsed": months_elapsed,
                        "months_remaining": months_remaining,
                        "immediate_latt": FOUNDER_IMMEDIATE_AMOUNT,
                        "vested_latt": vested_amount,
                        "locked_latt": locked_amount,
                        "monthly_release_latt": monthly_release,
                    }) } else { serde_json::Value::Null },
                }));
            }

            formatter::title("Wallet Balance");
            formatter::divider();
            formatter::key_value("Address", &address);
            formatter::key_value(
                "Balance",
                &format!("{} {}", format_lat_display(lat), TOKEN_SYMBOL),
            );
            formatter::key_value("Observed Block", &block_height.to_string());

            if is_founder {
                formatter::subheader("Founder Vesting Overview");
                formatter::key_value(
                    "Immediate",
                    &format!(
                        "{} {}",
                        format_lat_display(FOUNDER_IMMEDIATE_AMOUNT as f64 / LATT_PER_LAT as f64),
                        TOKEN_SYMBOL
                    ),
                );
                formatter::key_value(
                    "Vested",
                    &format!(
                        "{} {}",
                        format_lat_display(vested_amount as f64 / LATT_PER_LAT as f64),
                        TOKEN_SYMBOL
                    ),
                );
                formatter::key_value(
                    "Locked",
                    &format!(
                        "{} {}",
                        format_lat_display(locked_amount as f64 / LATT_PER_LAT as f64),
                        TOKEN_SYMBOL
                    ),
                );
                formatter::key_value(
                    "Monthly Release",
                    &format!(
                        "{} {}/month",
                        format_lat_display(monthly_release as f64 / LATT_PER_LAT as f64),
                        TOKEN_SYMBOL
                    ),
                );
                formatter::key_value("Months Remaining", &months_remaining.to_string());
            }
        }
        Err(e) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "wallet_balance",
                    "address": address,
                    "error": e.to_string(),
                    "rpc": rpc_url,
                }));
            }
            formatter::title("Wallet Balance");
            formatter::divider();
            formatter::key_value("Address", &address);
            formatter::error(&format!("RPC request failed: {e}"));
            formatter::note("Make sure the node RPC endpoint is running and reachable.");
        }
    }

    println!();
    Ok(())
}

fn format_lat_display(lat: f64) -> String {
    if lat >= 1000.0 {
        let whole = lat.trunc() as u128;
        let frac = ((lat.fract() * 100.0).round() as u64).min(99);
        format!("{}.{:02}", format_with_commas(whole), frac)
    } else if lat >= 1.0 {
        format!("{lat:.4}")
    } else {
        format!("{lat:.8}")
    }
}

fn format_with_commas(n: u128) -> String {
    let s = n.to_string();
    let mut result = String::new();
    for (i, c) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            result.push(',');
        }
        result.push(c);
    }
    result.chars().rev().collect()
}

fn default_wallet_config_path() -> Option<std::path::PathBuf> {
    dirs::home_dir().map(|home| {
        home.join(DEFAULT_WALLET_CONFIG_DIR)
            .join(DEFAULT_WALLET_CONFIG_FILE)
    })
}

fn resolve_address_input(address_or_wallet: &str) -> Result<String> {
    if Path::new(address_or_wallet).exists() {
        let keystore = Keystore::load_from_file(address_or_wallet)?;
        return Ok(keystore.address().to_string());
    }

    Address::from_base58(address_or_wallet).map_err(|_| anyhow!("Invalid address format"))?;
    Ok(address_or_wallet.to_string())
}

pub fn load_wallet(wallet_path: &str) -> Result<WalletAccount> {
    let path = Path::new(wallet_path);
    if !path.exists() {
        bail!("Wallet file not found: {wallet_path}");
    }

    let keystore = Keystore::load_from_file(path)?;
    let password = Password::new()
        .with_prompt("  Enter wallet password")
        .interact()?;

    let account = keystore.decrypt(&password)?;
    Ok(account)
}
