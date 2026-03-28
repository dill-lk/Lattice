//! Wallet command handlers

use anyhow::{anyhow, bail, Result};
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

    // Generate new account
    let account = WalletAccount::generate();
    let address = account.address().clone();

    // Get password with confirmation
    let password = Password::new()
        .with_prompt("Enter password for new wallet")
        .with_confirmation("Confirm password", "Passwords don't match")
        .interact()?;

    if password.len() < 8 {
        bail!("Password must be at least 8 characters");
    }

    // Encrypt and save
    let keystore = Keystore::encrypt(&account, &password)?;
    keystore.save_to_file(path)?;

    println!("✓ Created new wallet");
    println!("  Address: {}", address);
    println!("  Saved to: {}", output);
    println!();
    println!("⚠ IMPORTANT: Remember your password! It cannot be recovered.");

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

    // We need both public and secret keys - regenerate public from the original keypair
    // In a real implementation, we'd derive or store both
    // For now, prompt user that we need full keypair data
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

    // Mnemonic support would require bip39 crate
    // For now, we hash the mnemonic to derive key material
    // This is a simplified implementation
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

    // Load and verify the source keystore
    let source_keystore = Keystore::load_from_file(input_path)?;

    // Get password for source keystore
    let source_password = Password::new()
        .with_prompt("Enter password for source keystore")
        .interact()?;

    // Decrypt to verify password is correct
    let account = source_keystore.decrypt(&source_password)?;
    let address = account.address().clone();

    // Ask for new password
    let new_password = Password::new()
        .with_prompt("Enter new password for wallet")
        .with_confirmation("Confirm new password", "Passwords don't match")
        .interact()?;

    if new_password.len() < 8 {
        bail!("Password must be at least 8 characters");
    }

    // Create new keystore with new password
    let new_keystore = Keystore::encrypt(&account, &new_password)?;
    new_keystore.save_to_file(output_path)?;

    println!("✓ Imported wallet");
    println!("  Address: {}", address);
    println!("  Saved to: {}", output);

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

    println!("⚠ WARNING: You are about to export your private key.");
    println!("  Anyone with this key can access your funds!");
    println!();

    // Confirm export
    let confirmed = Confirm::new()
        .with_prompt("Are you sure you want to export your private key?")
        .default(false)
        .interact()?;

    if !confirmed {
        println!("Export cancelled.");
        return Ok(());
    }

    // Get password
    let password = Password::new()
        .with_prompt("Enter wallet password")
        .interact()?;

    // Decrypt
    let account = keystore.decrypt(&password)?;

    // Double confirmation
    let double_confirmed = Confirm::new()
        .with_prompt("FINAL WARNING: Display private key on screen?")
        .default(false)
        .interact()?;

    if !double_confirmed {
        println!("Export cancelled.");
        return Ok(());
    }

    // Display private key
    let secret_bytes = account.secret_key_bytes();
    println!();
    println!("Private Key: 0x{}", hex::encode(&*secret_bytes));
    println!();
    println!("⚠ Store this securely and never share it with anyone!");

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

    println!("Address: {}", keystore.address());
    println!("Keystore ID: {}", keystore.id());

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
        // Validate address format
        Address::from_base58(address_or_wallet)
            .map_err(|_| anyhow!("Invalid address format"))?;
        address_or_wallet.to_string()
    };

    // Query balance
    match client.get_balance(&address).await {
        Ok(balance) => {
            // Format balance (assuming 18 decimals like Ethereum)
            let whole = balance / 1_000_000_000_000_000_000u128;
            let frac = balance % 1_000_000_000_000_000_000u128;

            println!("Address: {}", address);
            println!("Balance: {}.{:018} LAT", whole, frac);
            println!("         ({} wei)", balance);
        }
        Err(e) => {
            // Connection error - show helpful message
            eprintln!("Failed to query balance: {}", e);
            eprintln!();
            eprintln!("Make sure the Lattice node is running at: {}", rpc_url);
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
        .with_prompt("Enter wallet password")
        .interact()?;

    let account = keystore.decrypt(&password)?;
    Ok(account)
}
