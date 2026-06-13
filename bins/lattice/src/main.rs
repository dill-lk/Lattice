//! Lattice - Unified blockchain command line tool
//!
//! Wires together the Full Node, Miner, and CLI Wallet into a single all-in-one binary.

pub mod cli;
pub mod miner;
pub mod node;

use clap::{Parser, Subcommand};

#[derive(Parser)]
#[command(name = "lattice")]
#[command(version = "0.1.0")]
#[command(about = "Lattice Unified Blockchain Platform - Node, Miner, Wallet, and Contracts")]
#[command(propagate_version = true)]
#[command(arg_required_else_help = true)]
struct Args {
    /// RPC endpoint URL for client commands
    #[arg(long, global = true, default_value = "http://127.0.0.1:8545")]
    rpc: String,

    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Run a full node (includes storage, RPC server, P2P network, and optional miner)
    Node(node::NodeArgs),

    /// Run the CPU-friendly standalone miner
    Miner(miner::MinerArgs),

    /// Manage wallet keypairs (create, import, export, balance, list, delete)
    Wallet {
        #[command(subcommand)]
        action: cli::WalletCommands,
    },

    /// Build, sign, and broadcast transactions (send, sign, broadcast, status, decode)
    Tx {
        #[command(subcommand)]
        action: cli::TxCommands,
    },

    /// Deploy and execute WebAssembly smart contracts (deploy, call)
    Contract {
        #[command(subcommand)]
        action: cli::ContractCommands,
    },

    /// Query the blockchain state (block, transaction, account)
    Query {
        #[command(subcommand)]
        action: cli::QueryCommands,
    },

    /// Show running node sync status
    Status,

    /// List connected peers of the running node
    Peers,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    let rpc_url = &args.rpc;

    match args.command {
        Commands::Node(node_args) => {
            node::run_node(node_args).await?;
        }
        Commands::Miner(miner_args) => {
            miner::run_miner(miner_args, rpc_url).await?;
        }
        other_cmd => {
            // Print the professional ASCII banner for client commands
            cli::formatter::print_banner();

            // Translate Commands to CliCommands
            let cli_cmd = match other_cmd {
                Commands::Wallet { action } => cli::CliCommands::Wallet { action },
                Commands::Tx { action } => cli::CliCommands::Tx { action },
                Commands::Contract { action } => cli::CliCommands::Contract { action },
                Commands::Query { action } => cli::CliCommands::Query { action },
                Commands::Status => cli::CliCommands::Status,
                Commands::Peers => cli::CliCommands::Peers,
                _ => unreachable!(),
            };

            cli::run_cli(cli_cmd, rpc_url).await?;
        }
    }

    Ok(())
}
