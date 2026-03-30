//! Wallet command handlers

use anyhow::{anyhow, bail, Result};
use colored::Colorize;
use dialoguer::{Confirm, Password};
use lattice_core::Address;
use lattice_wallet::{Keystore, WalletAccount};
use std::path::Path;

use crate::rpc_client::RpcClient;

/// Create a new wallet and save it to an encrypted keystore file
pub fn create_wallet(output: &str) -> Result<()> {
    let path = Path::new(output);

    // Check if file already exists
    if path.exists() {
        bail!("Wallet file already exists: {}", output);
    }

    // Print header
    println!();
    println!("  {}", "Create New Wallet".bold());
    println!("  {}", "─".repeat(45).dimmed());
    println!();

    // Generate new account
    let account = WalletAccount::generate();
    let address = account.address().clone();

    println!("  {} Generating quantum-safe keypair...", "●".cyan());

    // Get password with confirmation
    let password = Password::new()
        .with_prompt("  Enter password")
        .with_confirmation("  Confirm password", "  Passwords don't match")
        .interact()?;

    if password.len() < 8 {
        bail!("Password must be at least 8 characters");
    }

    println!("  {} Encrypting keystore...", "●".cyan());

    // Encrypt and save
    let keystore = Keystore::encrypt(&account, &password)?;
    keystore.save_to_file(path)?;

    println!();
    println!("  {}", "─".repeat(45).dimmed());
    println!("  {} Wallet created", "✓".green().bold());
    println!();
    println!("  {}  {}", "Address".dimmed(), address.to_string().white());
    println!("  {}    {}", "File".dimmed(), output.dimmed());
    println!();
    println!(
        "  {} {}",
        "!".yellow(),
        "Remember your password - it cannot be recovered".yellow()
    );
    println!();

    Ok(())
}

/// Import wallet from private key (hex-encoded)
pub fn import_from_private_key(private_key: &str, output: &str) -> Result<()> {
    let path = Path::new(output);

    if path.exists() {
        bail!("Wallet file already exists: {}", output);
    }

    // Parse private key
    let key_hex = private_key.strip_prefix("0x").unwrap_or(private_key);
    let secret_bytes = hex::decode(key_hex).map_err(|_| anyhow!("Invalid hex encoding"))?;

    // Reconstruct keypair from secret key
    let _secret = lattice_crypto::SecretKey::from_bytes(&secret_bytes)
        .map_err(|_| anyhow!("Invalid secret key format"))?;

    bail!(
        "Direct private key import is not supported for Dilithium keys. \
        Please use keystore file import instead."
    );
}

/// Import wallet from mnemonic phrase
pub fn import_from_mnemonic(_mnemonic: &str, output: &str) -> Result<()> {
    let path = Path::new(output);

    if path.exists() {
        bail!("Wallet file already exists: {}", output);
    }

    bail!(
        "Mnemonic import is not yet supported. \
        Please use keystore file import instead."
    );
}

/// Import wallet from an existing keystore file
pub fn import_from_keystore(keystore_path: &str, output: &str) -> Result<()> {
    let input_path = Path::new(keystore_path);
    let output_path = Path::new(output);

    if !input_path.exists() {
        bail!("Keystore file not found: {}", keystore_path);
    }

    if output_path.exists() {
        bail!("Output file already exists: {}", output);
    }

    println!();
    println!("  {}", "Import Wallet".bold());
    println!("  {}", "─".repeat(45).dimmed());
    println!();

    // Load and verify the source keystore
    let source_keystore = Keystore::load_from_file(input_path)?;

    // Get password for source keystore
    let source_password = Password::new()
        .with_prompt("  Enter source keystore password")
        .interact()?;

    // Decrypt to verify password is correct
    println!("  {} Decrypting...", "●".cyan());
    let account = source_keystore.decrypt(&source_password)?;
    let address = account.address().clone();

    // Ask for new password
    let new_password = Password::new()
        .with_prompt("  Enter new password")
        .with_confirmation("  Confirm new password", "  Passwords don't match")
        .interact()?;

    if new_password.len() < 8 {
        bail!("Password must be at least 8 characters");
    }

    println!("  {} Encrypting...", "●".cyan());

    // Create new keystore with new password
    let new_keystore = Keystore::encrypt(&account, &new_password)?;
    new_keystore.save_to_file(output_path)?;

    println!();
    println!("  {}", "─".repeat(45).dimmed());
    println!("  {} Wallet imported", "✓".green().bold());
    println!();
    println!("  {}  {}", "Address".dimmed(), address.to_string().white());
    println!("  {}    {}", "File".dimmed(), output.dimmed());
    println!();

    Ok(())
}

/// Export private key from wallet (dangerous operation)
pub fn export_private_key(wallet_path: &str) -> Result<()> {
    let path = Path::new(wallet_path);

    if !path.exists() {
        bail!("Wallet file not found: {}", wallet_path);
    }

    // Load keystore
    let keystore = Keystore::load_from_file(path)?;

    println!();
    println!(
        "  {} {}",
        "!".red().bold(),
        "WARNING: Private Key Export".red().bold()
    );
    println!("  {}", "─".repeat(45).dimmed());
    println!();
    println!("  Anyone with this key can access your funds.");
    println!();

    // Confirm export
    let confirmed = Confirm::new()
        .with_prompt("  Export private key?")
        .default(false)
        .interact()?;

    if !confirmed {
        println!("  Cancelled.");
        return Ok(());
    }

    // Get password
    let password = Password::new()
        .with_prompt("  Enter wallet password")
        .interact()?;

    // Decrypt
    let account = keystore.decrypt(&password)?;

    // Double confirmation
    let double_confirmed = Confirm::new()
        .with_prompt("  Display on screen?")
        .default(false)
        .interact()?;

    if !double_confirmed {
        println!("  Cancelled.");
        return Ok(());
    }

    // Display private key
    let secret_bytes = account.secret_key_bytes();
    println!();
    println!("  {}", "─".repeat(45).dimmed());
    println!("  {} 0x{}", "Key".dimmed(), hex::encode(&*secret_bytes));
    println!("  {}", "─".repeat(45).dimmed());
    println!();
    println!("  {} Store securely. Never share.", "!".yellow());
    println!();

    Ok(())
}

/// Show wallet address
pub fn show_address(wallet_path: &str) -> Result<()> {
    let path = Path::new(wallet_path);

    if !path.exists() {
        bail!("Wallet file not found: {}", wallet_path);
    }

    // Load keystore (no password needed for address)
    let keystore = Keystore::load_from_file(path)?;

    println!();
    println!("  {}  {}", "Address".dimmed(), keystore.address().to_string().white());
    println!("  {}       {}", "ID".dimmed(), keystore.id().dimmed());
    println!();

    Ok(())
}

/// Query wallet balance via RPC
pub async fn show_balance(address_or_wallet: &str, rpc_url: &str) -> Result<()> {
    use lattice_core::tokenomics::{
        BLOCKS_PER_MONTH, FOUNDER_IMMEDIATE_AMOUNT, FOUNDER_VESTING_AMOUNT,
        FOUNDER_WALLET_ADDRESS, LATT_PER_LAT, TOKEN_SYMBOL, VESTING_DURATION_MONTHS,
    };

    let client = RpcClient::new(rpc_url);

    // Determine if input is a wallet file or address
    let address = if Path::new(address_or_wallet).exists() {
        let keystore = Keystore::load_from_file(address_or_wallet)?;
        keystore.address().to_string()
    } else {
        Address::from_base58(address_or_wallet)
            .map_err(|_| anyhow!("Invalid address format"))?;
        address_or_wallet.to_string()
    };

    // Check if this is the founder wallet (has vesting)
    let is_founder = address == FOUNDER_WALLET_ADDRESS;

    // Query balance and block height
    let balance_result = client.get_balance(&address).await;
    let block_height = client.get_block_number().await.unwrap_or(0);

    match balance_result {
        Ok(balance) => {
            let lat = balance as f64 / LATT_PER_LAT as f64;

            println!();
            println!("  {}", "◈ LATTICE WALLET".cyan().bold());
            println!("  {}", "─".repeat(50).dimmed());
            println!();

            // Address (shortened for display)
            println!("  {}  {}", "Address".dimmed(), address.white());
            println!();

            // Main balance - big and prominent
            println!("  {}  {} {}", 
                "Balance".dimmed(),
                format_lat_display(lat).green().bold(),
                TOKEN_SYMBOL.cyan()
            );

            // Show vesting info for founder wallet
            if is_founder {
                println!();
                println!("  {}", "─".repeat(50).dimmed());
                println!("  {}", "Vesting Schedule".yellow().bold());
                println!();

                // Calculate vesting progress
                let months_elapsed = block_height / BLOCKS_PER_MONTH;
                let months_remaining = VESTING_DURATION_MONTHS.saturating_sub(months_elapsed);
                
                let vested_amount = if months_elapsed >= VESTING_DURATION_MONTHS {
                    FOUNDER_VESTING_AMOUNT
                } else {
                    (FOUNDER_VESTING_AMOUNT * months_elapsed as u128) / VESTING_DURATION_MONTHS as u128
                };
                let locked_amount = FOUNDER_VESTING_AMOUNT.saturating_sub(vested_amount);
                let monthly_release = FOUNDER_VESTING_AMOUNT / VESTING_DURATION_MONTHS as u128;

                let vested_lat = vested_amount as f64 / LATT_PER_LAT as f64;
                let locked_lat = locked_amount as f64 / LATT_PER_LAT as f64;
                let monthly_lat = monthly_release as f64 / LATT_PER_LAT as f64;
                let immediate_lat = FOUNDER_IMMEDIATE_AMOUNT as f64 / LATT_PER_LAT as f64;

                // Vesting progress bar
                let progress = if VESTING_DURATION_MONTHS > 0 {
                    (months_elapsed as f64 / VESTING_DURATION_MONTHS as f64).min(1.0)
                } else {
                    1.0
                };
                let bar_width = 30;
                let filled = (progress * bar_width as f64) as usize;
                let empty = bar_width - filled;
                
                println!("  {}  {}{} {:.0}%",
                    "Progress".dimmed(),
                    "█".repeat(filled).green(),
                    "░".repeat(empty).dimmed(),
                    progress * 100.0
                );
                println!();

                // Breakdown table
                println!("  {}  {} LAT  {}", 
                    "Immediate".dimmed(),
                    format_lat_display(immediate_lat).white(),
                    "(unlocked at genesis)".dimmed()
                );
                println!("  {}    {} LAT  {}",
                    "Vested".dimmed(),
                    format_lat_display(vested_lat).green(),
                    format!("(month {}/{})", months_elapsed.min(24), VESTING_DURATION_MONTHS).dimmed()
                );
                println!("  {}    {} LAT",
                    "Locked".dimmed(),
                    format_lat_display(locked_lat).yellow()
                );
                println!();
                
                if months_remaining > 0 {
                    println!("  {}   {} LAT/month",
                        "Release".dimmed(),
                        format_lat_display(monthly_lat).cyan()
                    );
                    println!("  {} {} months",
                        "Remaining".dimmed(),
                        format!("{}", months_remaining).white()
                    );
                } else {
                    println!("  {}  {}", "Status".dimmed(), "Fully vested ✓".green().bold());
                }
            }

            println!();
            println!("  {}", "─".repeat(50).dimmed());
            println!("  {}  Block #{}", "Network".dimmed(), format!("{}", block_height).dimmed());
            println!();
        }
        Err(e) => {
            println!();
            println!("  {} {}", "✗".red().bold(), "Connection Failed".red());
            println!();
            println!("  {}  {}", "Error".dimmed(), format!("{}", e).red());
            println!("  {}   {}", "Node".dimmed(), rpc_url.dimmed());
            println!();
            println!("  {}", "Make sure the node is running.".dimmed());
            println!();
        }
    }

    Ok(())
}

/// Format LAT for display with thousands separators
fn format_lat_display(lat: f64) -> String {
    if lat >= 1000.0 {
        // Large amounts: show with commas, 2 decimals
        let whole = lat.trunc() as u128;
        let frac = ((lat.fract() * 100.0).round() as u64).min(99);
        format!("{}.{:02}", format_with_commas(whole), frac)
    } else if lat >= 1.0 {
        // Medium amounts: 4 decimals
        format!("{:.4}", lat)
    } else {
        // Small amounts: 8 decimals
        format!("{:.8}", lat)
    }
}

/// Format number with comma separators
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

/// Load wallet account from keystore file with password prompt
pub fn load_wallet(wallet_path: &str) -> Result<WalletAccount> {
    let path = Path::new(wallet_path);

    if !path.exists() {
        bail!("Wallet file not found: {}", wallet_path);
    }

    let keystore = Keystore::load_from_file(path)?;

    let password = Password::new()
        .with_prompt("  Enter wallet password")
        .interact()?;

    let account = keystore.decrypt(&password)?;
    Ok(account)
}
