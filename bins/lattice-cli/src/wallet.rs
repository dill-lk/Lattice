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
    let client = RpcClient::new(rpc_url);

    // Determine if input is a wallet file or address
    let address = if Path::new(address_or_wallet).exists() {
        // It's a wallet file
        let keystore = Keystore::load_from_file(address_or_wallet)?;
        keystore.address().to_string()
    } else {
        // Assume it's an address
        Address::from_base58(address_or_wallet)
            .map_err(|_| anyhow!("Invalid address format"))?;
        address_or_wallet.to_string()
    };

    // Query balance
    match client.get_balance(&address).await {
        Ok(balance) => {
            use lattice_core::tokenomics::{LATT_PER_LAT, TOKEN_SYMBOL};
            let lat = balance as f64 / LATT_PER_LAT as f64;

            println!();
            println!("  {}  {}", "Address".dimmed(), address.white());
            println!(
                "  {}  {}",
                "Balance".dimmed(),
                format!("{:.8} {}", lat, TOKEN_SYMBOL).green().bold()
            );
            println!();
        }
        Err(e) => {
            eprintln!();
            eprintln!("  {} Failed to query: {}", "✗".red(), e);
            eprintln!("  {} Node: {}", " ".dimmed(), rpc_url.dimmed());
            eprintln!();
        }
    }

    Ok(())
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
