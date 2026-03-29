//! Lattice CLI - Command-line wallet and tools
//!
//! A comprehensive CLI for interacting with the Lattice blockchain.
//!
//! # Commands
//!
//! - `wallet` - Wallet management (create, import, export)
//! - `tx` - Transaction operations (send, status, decode)
//! - `query` - Query blockchain state (blocks, transactions, accounts)
//! - `node` - Node information (status, peers)

mod node;
mod query;
mod rpc_client;
mod transaction;
mod wallet;

use clap::{Parser, Subcommand};

/// Default RPC endpoint
const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8545";

#[derive(Parser)]
#[command(name = "lattice-cli")]
#[command(version, about = "Lattice blockchain CLI - Wallet and tools")]
#[command(propagate_version = true)]
#[command(arg_required_else_help = true)]
struct Args {
    /// RPC endpoint URL
    #[arg(long, global = true, default_value = DEFAULT_RPC_URL)]
    rpc: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
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
    /// Query blockchain state
    Query {
        #[command(subcommand)]
        action: QueryCommands,
    },
    /// Node information
    Node {
        #[command(subcommand)]
        action: NodeCommands,
    },
}

#[derive(Subcommand)]
enum WalletCommands {
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
}

#[derive(Subcommand)]
enum ImportSource {
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

#[derive(Subcommand)]
enum TxCommands {
    /// Build, sign, and broadcast a transfer transaction
    Send {
        /// Recipient address (base58)
        #[arg(short, long)]
        to: String,

        /// Amount to send in LAT (or wei with --wei flag)
        #[arg(short, long)]
        amount: String,

        /// Interpret amount as wei instead of LAT
        #[arg(long)]
        wei: bool,

        /// Transaction fee in LAT (or wei with --wei flag)
        #[arg(short, long, default_value = "0.001")]
        fee: String,

        /// Gas limit for the transaction
        #[arg(short, long)]
        gas_limit: Option<u64>,

        /// Wallet file path
        #[arg(short, long, default_value = "wallet.json")]
        wallet: String,
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

#[derive(Subcommand)]
enum QueryCommands {
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

#[derive(Subcommand)]
enum NodeCommands {
    /// Show node sync status
    Status,

    /// List connected peers
    Peers,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let rpc_url = &args.rpc;

    match args.command {
        Commands::Wallet { action } => match action {
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
        },

        Commands::Tx { action } => match action {
            TxCommands::Send {
                to,
                amount,
                wei,
                fee,
                gas_limit,
                wallet: wallet_path,
            } => {
                let amount_wei = parse_amount(&amount, wei)?;
                let fee_wei = parse_amount(&fee, wei)?;

                transaction::send_transaction(
                    &wallet_path,
                    &to,
                    amount_wei,
                    fee_wei,
                    gas_limit,
                    rpc_url,
                )
                .await?;
            }

            TxCommands::Status { hash } => {
                transaction::check_status(&hash, rpc_url).await?;
            }

            TxCommands::Decode { raw } => {
                transaction::decode_transaction(&raw)?;
            }
        },

        Commands::Query { action } => match action {
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

        Commands::Node { action } => match action {
            NodeCommands::Status => {
                node::show_status(rpc_url).await?;
            }

            NodeCommands::Peers => {
                node::list_peers(rpc_url).await?;
            }
        },
    }

    Ok(())
}

/// Parse amount string to wei (smallest unit)
/// Supports LAT (with 18 decimals) or wei directly
fn parse_amount(s: &str, is_wei: bool) -> anyhow::Result<u128> {
    if is_wei {
        // Parse as wei directly
        s.parse::<u128>()
            .map_err(|_| anyhow::anyhow!("Invalid wei amount: {}", s))
    } else {
        // Parse as LAT with decimals
        parse_lat_amount(s)
    }
}

/// Parse LAT amount string (e.g., "1.5" LAT = 1.5 * 10^8 Latt)
fn parse_lat_amount(s: &str) -> anyhow::Result<u128> {
    // 8 decimals for LAT (like Bitcoin)
    const DECIMALS: u32 = 8;
    const MULTIPLIER: u128 = 100_000_000; // 10^8

    if let Some(dot_pos) = s.find('.') {
        let whole_str = &s[..dot_pos];
        let frac_str = &s[dot_pos + 1..];

        // Validate no extra dots
        if frac_str.contains('.') {
            return Err(anyhow::anyhow!("Invalid amount format: {}", s));
        }

        // Parse whole part
        let whole: u128 = if whole_str.is_empty() {
            0
        } else {
            whole_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid whole part: {}", whole_str))?
        };

        // Parse fractional part (pad or truncate to 8 digits)
        let frac_padded = if frac_str.len() >= DECIMALS as usize {
            frac_str[..DECIMALS as usize].to_string()
        } else {
            format!("{:0<width$}", frac_str, width = DECIMALS as usize)
        };

        let frac: u128 = frac_padded
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid fractional part: {}", frac_str))?;

        // Combine: whole * 10^8 + frac
        let result = whole
            .checked_mul(MULTIPLIER)
            .and_then(|w| w.checked_add(frac))
            .ok_or_else(|| anyhow::anyhow!("Amount overflow"))?;

        Ok(result)
    } else {
        // No decimal point - parse as whole LAT
        let whole: u128 = s
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid amount: {}", s))?;

        whole
            .checked_mul(MULTIPLIER)
            .ok_or_else(|| anyhow::anyhow!("Amount overflow"))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::tokenomics::LATT_PER_LAT;

    #[test]
    fn test_parse_lat_amount_whole() {
        // 1 LAT = 100_000_000 Latt (8 decimals)
        assert_eq!(parse_lat_amount("1").unwrap(), LATT_PER_LAT);
        assert_eq!(parse_lat_amount("100").unwrap(), 100 * LATT_PER_LAT);
        assert_eq!(parse_lat_amount("0").unwrap(), 0);
    }

    #[test]
    fn test_parse_lat_amount_decimal() {
        // 1.5 LAT = 150_000_000 Latt
        assert_eq!(parse_lat_amount("1.5").unwrap(), 150_000_000);
        // 0.001 LAT = 100_000 Latt
        assert_eq!(parse_lat_amount("0.001").unwrap(), 100_000);
        // 0.00000001 LAT = 1 Latt (smallest unit)
        assert_eq!(parse_lat_amount("0.00000001").unwrap(), 1);
    }

    #[test]
    fn test_parse_lat_amount_edge_cases() {
        // .5 LAT = 50_000_000 Latt
        assert_eq!(parse_lat_amount(".5").unwrap(), 50_000_000);
        assert!(parse_lat_amount("1.2.3").is_err());
        assert!(parse_lat_amount("abc").is_err());
    }

    #[test]
    fn test_parse_amount_wei_flag() {
        // Raw Latt mode
        assert_eq!(parse_amount("1000000", true).unwrap(), 1000000);
        // LAT mode (converted to Latt)
        assert_eq!(parse_amount("1", false).unwrap(), LATT_PER_LAT);
    }
}
