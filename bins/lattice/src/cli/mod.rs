//! Lattice CLI - Command-line wallet and tools module
//!
//! A comprehensive CLI for interacting with the Lattice blockchain.

pub mod formatter;
pub mod node;
pub mod query;
pub mod rpc_client;
pub mod transaction;
pub mod wallet;

use clap::Subcommand;

#[derive(Subcommand, Debug, Clone)]
pub enum CliCommands {
    /// Wallet management commands
    Wallet {
        #[command(subcommand)]
        action: WalletCommands,
    },
    /// Transaction operations
    Tx {
        #[command(subcommand)]
        action: TxCommands,
    },
    /// Smart Contract operations
    Contract {
        #[command(subcommand)]
        action: ContractCommands,
    },
    /// Query blockchain state
    Query {
        #[command(subcommand)]
        action: QueryCommands,
    },
    /// Show node sync status
    Status,
    /// List connected peers
    Peers,
}

#[derive(Subcommand, Debug, Clone)]
pub enum WalletCommands {
    /// Create a new wallet with encrypted keystore
    Create {
        /// Output file path for the keystore
        #[arg(short, long, default_value = "wallet.json")]
        output: String,
    },

    /// Import wallet from keystore, private key, or mnemonic
    Import {
        /// Import source type
        #[command(subcommand)]
        source: ImportSource,
    },

    /// Export private key (requires password confirmation)
    Export {
        /// Wallet file path
        #[arg(short, long, default_value = "wallet.json")]
        wallet: String,
    },

    /// Show wallet address
    Address {
        /// Wallet file path
        #[arg(short, long, default_value = "wallet.json")]
        wallet: String,
    },

    /// Query wallet balance via RPC
    Balance {
        /// Wallet file path or address (base58)
        address: String,
    },

    /// List all keystores in the current directory
    List,

    /// Delete a wallet keystore file
    Delete {
        /// Keystore file path to delete
        wallet: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ImportSource {
    /// Import from existing keystore file
    Keystore {
        /// Source keystore file path
        file: String,
        /// Output file path
        #[arg(short, long, default_value = "wallet.json")]
        output: String,
    },

    /// Import from private key (hex-encoded)
    PrivateKey {
        /// Private key in hex format (with or without 0x prefix)
        key: String,
        /// Output file path
        #[arg(short, long, default_value = "wallet.json")]
        output: String,
    },

    /// Import from mnemonic phrase
    Mnemonic {
        /// BIP-39 mnemonic phrase (12 or 24 words)
        phrase: String,
        /// Output file path
        #[arg(short, long, default_value = "wallet.json")]
        output: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum TxCommands {
    /// Build, sign, and broadcast a transfer transaction
    Send {
        /// Recipient address (base58)
        #[arg(short, long)]
        to: String,

        /// Amount to send in LAT (or Latt with --latt flag)
        #[arg(short, long)]
        amount: String,

        /// Interpret amount as Latt (raw subunit) instead of LAT
        #[arg(long)]
        latt: bool,

        /// Transaction fee in LAT (or Latt with --latt flag)
        #[arg(short, long, default_value = "0.001")]
        fee: String,

        /// Gas limit for the transaction
        #[arg(short, long)]
        gas_limit: Option<u64>,

        /// Wallet file path
        #[arg(short, long, default_value = "wallet.json")]
        wallet: String,
    },

    /// Sign a transaction offline and display raw hex representation
    Sign {
        /// Recipient address (base58)
        #[arg(short, long)]
        to: String,

        /// Amount to send in LAT (or Latt with --latt flag)
        #[arg(short, long)]
        amount: String,

        /// Interpret amount as Latt instead of LAT
        #[arg(long)]
        latt: bool,

        /// Transaction fee in LAT (or Latt with --latt flag)
        #[arg(short, long, default_value = "0.001")]
        fee: String,

        /// Sender Account Nonce
        #[arg(short, long)]
        nonce: u64,

        /// Gas limit for the transaction
        #[arg(short, long)]
        gas_limit: Option<u64>,

        /// Optional raw hex payload data
        #[arg(long)]
        data: Option<String>,

        /// Wallet file path
        #[arg(short, long, default_value = "wallet.json")]
        wallet: String,
    },

    /// Broadcast a signed transaction hex to the network
    Broadcast {
        /// Raw transaction hex string (with or without 0x prefix)
        hex: String,
    },

    /// Check transaction status by hash
    Status {
        /// Transaction hash (hex, with or without 0x prefix)
        hash: String,
    },

    /// Decode a raw transaction (hex-encoded)
    Decode {
        /// Raw transaction in hex format
        raw: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum ContractCommands {
    /// Deploy a compiled WASM smart contract
    Deploy {
        /// Path to compile WebAssembly contract file (*.wasm)
        #[arg(short, long)]
        wasm: String,

        /// Transaction fee in LAT
        #[arg(short, long, default_value = "0.005")]
        fee: String,

        /// Gas limit for contract deployment
        #[arg(short, long)]
        gas_limit: Option<u64>,

        /// Wallet file path
        #[arg(short, long, default_value = "wallet.json")]
        wallet: String,
    },

    /// Invoke a smart contract method
    Call {
        /// Deployed smart contract address (base58)
        #[arg(short, long)]
        address: String,

        /// Method name to invoke
        #[arg(short, long)]
        method: String,

        /// Arguments as hex string payload (optional)
        #[arg(long)]
        args: Option<String>,

        /// Value/amount to transfer to the contract in LAT (optional)
        #[arg(long, default_value = "0")]
        amount: String,

        /// Transaction fee in LAT
        #[arg(short, long, default_value = "0.002")]
        fee: String,

        /// Gas limit for contract invocation
        #[arg(short, long)]
        gas_limit: Option<u64>,

        /// Wallet file path
        #[arg(short, long, default_value = "wallet.json")]
        wallet: String,
    },
}

#[derive(Subcommand, Debug, Clone)]
pub enum QueryCommands {
    /// Get block by number or hash
    Block {
        /// Block number, hash, or tag (latest, earliest)
        id: String,

        /// Include full transaction objects
        #[arg(short = 't', long)]
        include_txs: bool,
    },

    /// Get transaction by hash
    Tx {
        /// Transaction hash (hex, with or without 0x prefix)
        hash: String,
    },

    /// Get account information (balance, nonce)
    Account {
        /// Account address (base58)
        address: String,
    },
}

pub async fn run_cli(command: CliCommands, rpc_url: &str) -> anyhow::Result<()> {
    match command {
        CliCommands::Wallet { action } => match action {
            WalletCommands::Create { output } => {
                wallet::create_wallet(&output)?;
            }
            WalletCommands::Import { source } => match source {
                ImportSource::Keystore { file, output } => {
                    wallet::import_from_keystore(&file, &output)?;
                }
                ImportSource::PrivateKey { key, output } => {
                    wallet::import_from_private_key(&key, &output)?;
                }
                ImportSource::Mnemonic { phrase, output } => {
                    wallet::import_from_mnemonic(&phrase, &output)?;
                }
            },
            WalletCommands::Export { wallet: wallet_path } => {
                wallet::export_private_key(&wallet_path)?;
            }
            WalletCommands::Address { wallet: wallet_path } => {
                wallet::show_address(&wallet_path)?;
            }
            WalletCommands::Balance { address } => {
                wallet::show_balance(&address, rpc_url).await?;
            }
            WalletCommands::List => {
                wallet::list_wallets()?;
            }
            WalletCommands::Delete { wallet: wallet_path } => {
                wallet::delete_wallet(&wallet_path)?;
            }
        },

        CliCommands::Tx { action } => match action {
            TxCommands::Send {
                to,
                amount,
                latt,
                fee,
                gas_limit,
                wallet: wallet_path,
            } => {
                let amount_latt = parse_amount(&amount, latt)?;
                let fee_latt = parse_amount(&fee, latt)?;

                transaction::send_transaction(
                    &wallet_path,
                    &to,
                    amount_latt,
                    fee_latt,
                    gas_limit,
                    rpc_url,
                )
                .await?;
            }
            TxCommands::Sign {
                to,
                amount,
                latt,
                fee,
                nonce,
                gas_limit,
                data,
                wallet: wallet_path,
            } => {
                let amount_latt = parse_amount(&amount, latt)?;
                let fee_latt = parse_amount(&fee, latt)?;

                transaction::sign_transaction(
                    &wallet_path,
                    &to,
                    amount_latt,
                    fee_latt,
                    nonce,
                    gas_limit,
                    data.as_deref(),
                )?;
            }
            TxCommands::Broadcast { hex } => {
                transaction::broadcast_transaction(&hex, rpc_url).await?;
            }
            TxCommands::Status { hash } => {
                transaction::check_status(&hash, rpc_url).await?;
            }
            TxCommands::Decode { raw } => {
                transaction::decode_transaction(&raw)?;
            }
        },

        CliCommands::Contract { action } => match action {
            ContractCommands::Deploy {
                wasm,
                fee,
                gas_limit,
                wallet: wallet_path,
            } => {
                let fee_latt = parse_amount(&fee, false)?;
                transaction::deploy_contract(&wallet_path, &wasm, fee_latt, gas_limit, rpc_url).await?;
            }
            ContractCommands::Call {
                address,
                method,
                args,
                amount,
                fee,
                gas_limit,
                wallet: wallet_path,
            } => {
                let amount_latt = parse_amount(&amount, false)?;
                let fee_latt = parse_amount(&fee, false)?;
                transaction::call_contract(
                    &wallet_path,
                    &address,
                    &method,
                    args.as_deref(),
                    amount_latt,
                    fee_latt,
                    gas_limit,
                    rpc_url,
                )
                .await?;
            }
        },

        CliCommands::Query { action } => match action {
            QueryCommands::Block { id, include_txs } => {
                query::get_block(&id, include_txs, rpc_url).await?;
            }
            QueryCommands::Tx { hash } => {
                query::get_transaction(&hash, rpc_url).await?;
            }
            QueryCommands::Account { address } => {
                query::get_account(&address, rpc_url).await?;
            }
        },

        CliCommands::Status => {
            node::show_status(rpc_url).await?;
        }
        CliCommands::Peers => {
            node::list_peers(rpc_url).await?;
        }
    }

    Ok(())
}

fn parse_amount(s: &str, is_latt: bool) -> anyhow::Result<u128> {
    if is_latt {
        s.parse::<u128>()
            .map_err(|_| anyhow::anyhow!("Invalid Latt amount: {}", s))
    } else {
        parse_lat_amount(s)
    }
}

fn parse_lat_amount(s: &str) -> anyhow::Result<u128> {
    const DECIMALS: u32 = lattice_core::tokenomics::DECIMALS as u32;
    const MULTIPLIER: u128 = lattice_core::tokenomics::LATT_PER_LAT;

    if let Some(dot_pos) = s.find('.') {
        let whole_str = &s[..dot_pos];
        let frac_str = &s[dot_pos + 1..];

        if frac_str.contains('.') {
            return Err(anyhow::anyhow!("Invalid amount format: {}", s));
        }

        let whole: u128 = if whole_str.is_empty() {
            0
        } else {
            whole_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid whole part: {}", whole_str))?
        };

        let frac_padded = if frac_str.len() >= DECIMALS as usize {
            frac_str[..DECIMALS as usize].to_string()
        } else {
            format!("{:0<width$}", frac_str, width = DECIMALS as usize)
        };

        let frac: u128 = frac_padded
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid fractional part: {}", frac_str))?;

        let result = whole
            .checked_mul(MULTIPLIER)
            .and_then(|w| w.checked_add(frac))
            .ok_or_else(|| anyhow::anyhow!("Amount overflow"))?;

        Ok(result)
    } else {
        let whole: u128 = s
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid amount: {}", s))?;

        whole
            .checked_mul(MULTIPLIER)
            .ok_or_else(|| anyhow::anyhow!("Amount overflow"))
    }
}
