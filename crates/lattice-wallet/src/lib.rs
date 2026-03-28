//! Lattice Wallet - Key management and transaction building
//!
//! Provides secure key storage and transaction signing.
//!
//! # Features
//!
//! - [`WalletAccount`] - Account with keypair and nonce tracking
//! - [`Keystore`] - Encrypted key storage with Argon2 + AES-GCM
//! - [`TransactionBuilder`] - Fluent API for building transactions
//!
//! # Example
//!
//! ```ignore
//! use lattice_wallet::{WalletAccount, TransactionBuilder, Keystore};
//! use lattice_core::Address;
//!
//! // Generate new account
//! let mut account = WalletAccount::generate();
//!
//! // Build and sign a transaction
//! let tx = TransactionBuilder::transfer()
//!     .to(recipient_address)
//!     .amount(1000)
//!     .fee(10)
//!     .build(&mut account)?;
//!
//! // Save encrypted keystore
//! let keystore = Keystore::encrypt(&account, "password")?;
//! keystore.save_to_file("wallet.json")?;
//! ```

mod account;
mod error;
mod keystore;
mod transfer;

pub use account::WalletAccount;
pub use error::{Result, WalletError};
pub use keystore::{CipherParams, CryptoParams, KdfParams, Keystore};
pub use transfer::TransactionBuilder;
