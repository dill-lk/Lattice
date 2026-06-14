//! Lattice - Unified blockchain command line tool
//!
//! Official all-in-one executable for the Lattice ecosystem.
//! It combines node, miner, wallet, transactions, queries, and contract tools
//! into a single operator experience.

use std::fs;
use std::path::Path;

use clap::{ArgAction, CommandFactory, Parser, Subcommand, ValueEnum};
use lattice::{cli, miner, node, CLI_AFTER_HELP};
use lattice_wallet::Keystore;
use colored::Colorize;

const DEFAULT_RPC_URL: &str = "http://127.0.0.1:8545";

#[derive(Parser, Debug)]
#[command(name = "lattice")]
#[command(disable_help_flag = true)]
#[command(disable_version_flag = true)]
#[command(arg_required_else_help = false)]
#[command(after_help = CLI_AFTER_HELP)]
struct Args {
    /// Print the minimalist help layout
    #[arg(short = 'h', long = "help", global = true, action = ArgAction::SetTrue)]
    help_flag: bool,

    /// Print version information
    #[arg(short = 'v', long = "version", global = true, action = ArgAction::SetTrue)]
    version_flag: bool,

    /// Emit structured JSON output instead of human-oriented terminal cards
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    json: bool,

    /// Reduce decorative terminal output
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    quiet: bool,

    /// Enable extra explanatory output where available
    #[arg(long, global = true, action = ArgAction::SetTrue)]
    verbose: bool,

    /// RPC endpoint used by wallet, query, and status operations
    #[arg(long, global = true, default_value = DEFAULT_RPC_URL)]
    rpc: String,

    /// Boot the full node daemon using default local configuration
    #[arg(long, action = ArgAction::SetTrue)]
    node: bool,

    /// Start mining using the default wallet as the coinbase target
    #[arg(long, value_name = "threads")]
    mine: Option<usize>,

    /// Create a brand-new encrypted Dilithium wallet at ./wallet.json
    #[arg(long = "wallet-new", action = ArgAction::SetTrue)]
    wallet_new: bool,

    /// Query wallet balance for the given address or wallet file
    #[arg(long, value_name = "address")]
    balance: Option<String>,

    /// Quick-send LAT to a recipient using ./wallet.json
    #[arg(long, value_name = "address")]
    send: Option<String>,

    /// Amount used with --send
    #[arg(long, value_name = "tokens")]
    amount: Option<String>,

    #[command(subcommand)]
    command: Option<Commands>,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Run a full blockchain node with storage, RPC, networking, and optional mining
    #[command(alias = "n")]
    Node(node::NodeArgs),

    /// Run the standalone CPU miner against a node RPC endpoint
    #[command(alias = "m")]
    Miner(miner::MinerArgs),

    /// Create, inspect, import, export, and manage wallets
    #[command(alias = "w")]
    Wallet {
        #[command(subcommand)]
        action: cli::WalletCommands,
    },

    /// Build, sign, broadcast, decode, and inspect transactions
    #[command(alias = "t")]
    Tx {
        #[command(subcommand)]
        action: cli::TxCommands,
    },

    /// Deploy and call WASM smart contracts
    #[command(alias = "c")]
    Contract {
        #[command(subcommand)]
        action: cli::ContractCommands,
    },

    /// Query blocks, accounts, and transactions from the chain
    #[command(alias = "q")]
    Query {
        #[command(subcommand)]
        action: cli::QueryCommands,
    },

    /// Show node sync / health status from the configured RPC endpoint
    Status,

    /// Show peers visible to the configured node RPC endpoint
    Peers,

    /// Show chain tip, latest block difficulty, and related chain status
    Chain,

    /// Show pending-transaction / mempool information
    Mempool,

    /// Generate shell completions for the unified CLI
    Completion {
        /// Shell to generate completions for
        shell: CompletionShell,
    },

    /// Generate a starter node configuration interactively
    Config {
        #[command(subcommand)]
        action: ConfigCommands,
    },

    /// Run a local diagnostics pass for wallet, RPC, and data-directory setup
    Doctor,
}

#[derive(Subcommand, Debug)]
enum ConfigCommands {
    /// Create a configuration file interactively
    Init {
        /// Output file path (defaults to ./node.toml)
        #[arg(long, default_value = "node.toml")]
        path: String,
    },
}

#[derive(Clone, Debug, ValueEnum)]
enum CompletionShell {
    Bash,
    Zsh,
    Fish,
    PowerShell,
    Elvish,
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();
    cli::output::configure(args.json, args.quiet, args.verbose);

    if args.help_flag {
        print_minimal_help();
        return Ok(());
    }

    if args.version_flag {
        print_version();
        return Ok(());
    }

    ensure_single_top_level_action(&args)?;

    if !has_any_action(&args) {
        print_snapshot(&args.rpc).await?;
        return Ok(());
    }

    if args.node {
        return node::run_node(default_node_args()).await;
    }

    if let Some(threads) = args.mine {
        let coinbase = resolve_default_wallet_address(&cli::wallet::get_default_wallet_path())?;
        let mut node_args = default_node_args();
        node_args.mine = true;
        node_args.mining_threads = Some(threads.max(1));
        node_args.coinbase = Some(coinbase);
        return node::run_node(node_args).await;
    }

    if args.wallet_new {
        cli::output::print_banner_if_needed();
        let default_wallet = cli::wallet::get_default_wallet_path();
        return cli::wallet::create_wallet(&default_wallet);
    }

    if let Some(address) = &args.balance {
        cli::output::print_banner_if_needed();
        return cli::wallet::show_balance(address, &args.rpc).await;
    }

    if let Some(recipient) = &args.send {
        let amount = args
            .amount
            .as_deref()
            .ok_or_else(|| anyhow::anyhow!("--send requires --amount <tokens>"))?;

        cli::output::print_banner_if_needed();
        let default_wallet = cli::wallet::get_default_wallet_path();
        return cli::transaction::send_transaction(
            &default_wallet,
            recipient,
            parse_lat_amount(amount)?,
            parse_lat_amount("0.001")?,
            None,
            &args.rpc,
        )
        .await;
    }

    if let Some(command) = args.command {
        return run_subcommand(command, &args.rpc).await;
    }

    Ok(())
}

async fn run_subcommand(command: Commands, rpc_url: &str) -> anyhow::Result<()> {
    match command {
        Commands::Node(node_args) => node::run_node(node_args).await,
        Commands::Miner(miner_args) => run_miner_with_auto_node(miner_args, rpc_url).await,
        Commands::Doctor => {
            cli::output::print_banner_if_needed();
            cli::doctor::run_doctor(rpc_url).await
        }
        Commands::Chain => {
            cli::output::print_banner_if_needed();
            cli::node::show_chain(rpc_url).await
        }
        Commands::Mempool => {
            cli::output::print_banner_if_needed();
            cli::node::show_mempool(rpc_url).await
        }
        Commands::Completion { shell } => print_completion(shell),
        Commands::Config { action } => {
            cli::output::print_banner_if_needed();
            match action {
                ConfigCommands::Init { path } => cli::doctor::run_config_wizard(&path),
            }
        }
        other_cmd => {
            cli::output::print_banner_if_needed();

            let cli_cmd = match other_cmd {
                Commands::Wallet { action } => cli::CliCommands::Wallet { action },
                Commands::Tx { action } => cli::CliCommands::Tx { action },
                Commands::Contract { action } => cli::CliCommands::Contract { action },
                Commands::Query { action } => cli::CliCommands::Query { action },
                Commands::Status => cli::CliCommands::Status,
                Commands::Peers => cli::CliCommands::Peers,
                Commands::Node(_)
                | Commands::Miner(_)
                | Commands::Doctor
                | Commands::Chain
                | Commands::Mempool
                | Commands::Completion { .. }
                | Commands::Config { .. } => unreachable!(),
            };

            cli::run_cli(cli_cmd, rpc_url).await
        }
    }
}

async fn run_miner_with_auto_node(
    miner_args: miner::MinerArgs,
    rpc_url: &str,
) -> anyhow::Result<()> {
    let local_default_rpc = rpc_url == DEFAULT_RPC_URL;

    if local_default_rpc {
        let rpc = cli::rpc_client::RpcClient::new(rpc_url);
        if rpc.get_block_number().await.is_err() {
            println!();
            println!(
                "{}",
                "▲ lattice miner — no local node detected, switching to integrated miner-node mode"
                    .bold()
                    .cyan()
            );
            println!("{}", "──────────────────────────────────────────────────────────────".dimmed());
            println!("  {}  {}", "network".dimmed(), miner_args.network.as_str());
            println!("  {}  {}", "rpc".dimmed(), rpc_url);
            println!("  {}  {}", "target".dimmed(), miner_args.coinbase.as_str());
            println!("  {}  {}", "workers".dimmed(), miner_args.threads.max(1));
            println!();
            println!(
                "  {} auto-starting a local node so mining works out of the box",
                "•".blue()
            );
            println!(
                "  {} once networking is fully hardened, this path will also inherit the normal peer/bootstrap flow",
                "•".blue()
            );
            println!();

            let mut node_args = default_node_args();
            node_args.mine = true;
            node_args.mining_threads = Some(miner_args.threads.max(1));
            node_args.coinbase = Some(miner_args.coinbase.clone());
            node_args.network = Some(miner_args.network.clone());
            node_args.light = miner_args.light;
            return node::run_node(node_args).await;
        }
    }

    miner::run_miner(miner_args, rpc_url).await
}

fn default_node_args() -> node::NodeArgs {
    node::NodeArgs {
        config: None,
        datadir: None,
        network: None,
        rpc_port: None,
        rpc_host: None,
        no_rpc: false,
        p2p_port: None,
        mine: false,
        mining_threads: None,
        coinbase: None,
        log_level: None,
        bootnodes: None,
        init: false,
        light: false,
    }
}

fn has_any_action(args: &Args) -> bool {
    args.node
        || args.mine.is_some()
        || args.wallet_new
        || args.balance.is_some()
        || args.send.is_some()
        || args.command.is_some()
}

fn ensure_single_top_level_action(args: &Args) -> anyhow::Result<()> {
    let mut count = 0usize;
    if args.node {
        count += 1;
    }
    if args.mine.is_some() {
        count += 1;
    }
    if args.wallet_new {
        count += 1;
    }
    if args.balance.is_some() {
        count += 1;
    }
    if args.send.is_some() {
        count += 1;
    }
    if args.command.is_some() {
        count += 1;
    }

    if count > 1 {
        anyhow::bail!(
            "Use one primary action at a time. Try `lattice --help` for the minimalist layout."
        );
    }

    if args.amount.is_some() && args.send.is_none() {
        anyhow::bail!("--amount is only valid together with --send <address>");
    }

    Ok(())
}

fn resolve_default_wallet_address(path: &str) -> anyhow::Result<String> {
    let wallet_path = Path::new(path);
    if !wallet_path.exists() {
        anyhow::bail!(
            "Default wallet not found at {}. Create one first with `lattice --wallet-new`.",
            path
        );
    }

    let keystore = Keystore::load_from_file(wallet_path)
        .map_err(|e| anyhow::anyhow!("Failed to load default wallet {}: {}", path, e))?;
    Ok(keystore.address().to_string())
}

fn print_version() {
    println!(
        "▲ lattice v{} — post-quantum sharded ledger",
        env!("CARGO_PKG_VERSION")
    );
}

fn print_completion(shell: CompletionShell) -> anyhow::Result<()> {
    let mut command = Args::command();
    let shell = match shell {
        CompletionShell::Bash => clap_complete::Shell::Bash,
        CompletionShell::Zsh => clap_complete::Shell::Zsh,
        CompletionShell::Fish => clap_complete::Shell::Fish,
        CompletionShell::PowerShell => clap_complete::Shell::PowerShell,
        CompletionShell::Elvish => clap_complete::Shell::Elvish,
    };
    clap_complete::generate(shell, &mut command, "lattice", &mut std::io::stdout());
    Ok(())
}

fn print_minimal_help() {
    println!(
        "▲ lattice v{} — post-quantum sharded ledger\n",
        env!("CARGO_PKG_VERSION")
    );
    println!(" usage\n");
    println!("   lattice [options]\n");
    println!("   lattice [commands]\n");
    println!(" core options\n");
    println!("   --node               boot up the full node network daemon stream");
    println!("   --mine <threads>     start local mining using the default wallet coinbase");
    println!("   --wallet-new         generate a brand new crystals-dilithium3 keypair wallet");
    println!("   --balance <address>  check total balance of a specific wallet public address\n");
    println!(" state options\n");
    println!("   --send <address>     receiver address for a quick transaction transfer [requires --amount]");
    println!("   --amount <tokens>    value token amount to transfer over shard channels [requires --send]\n");
    println!(" global flags\n");
    println!("   --json               emit machine-friendly JSON output where supported");
    println!("   --quiet              reduce decorative terminal output");
    println!("   --verbose            enable extra explanatory output where supported");
    println!("   -h, --help           print this minimalist command execution layout");
    println!("   -v, --version        print current version info\n");
    println!(" command paths\n");
    println!("   lattice node ...     advanced full-node controls");
    println!("   lattice miner ...    standalone miner controls");
    println!("   lattice wallet ...   wallet management toolkit");
    println!("   lattice tx ...       transaction build / sign / inspect toolkit");
    println!("   lattice query ...    low-level query toolkit");
    println!("   lattice contract ... wasm contract toolkit");
    println!("   lattice chain        chain tip / difficulty / latest block summary");
    println!("   lattice mempool      pending transaction summary");
    println!("   lattice doctor       local diagnostics and setup checks");
    println!("   lattice config init  interactive node config wizard");
    println!("   lattice completion   shell completion generator\n");
    println!(" examples\n");
    println!("   lattice --wallet-new");
    println!("   lattice --mine 4");
    println!("   lattice --json status");
    println!("   lattice wallet default set wallet.json");
    println!("   lattice config init --path node.toml\n");
}

async fn print_snapshot(rpc_url: &str) -> anyhow::Result<()> {
    let client = cli::rpc_client::RpcClient::new(rpc_url);
    let storage_gb = estimate_storage_gb();

    let sync = client.get_sync_status().await.ok();
    let peers = client.get_peers().await.unwrap_or_default();
    let height = sync.as_ref().map(|s| s.current_block).unwrap_or(0);
    let status = if sync.as_ref().map(|s| !s.syncing).unwrap_or(false) {
        "synchronized"
    } else {
        "offline"
    };

    if cli::output::json_enabled() {
        return cli::output::emit_json(serde_json::json!({
            "app": "lattice",
            "version": env!("CARGO_PKG_VERSION"),
            "status": status,
            "peers": peers.len(),
            "height": height,
            "hashrate": null,
            "storage_gb": storage_gb,
            "rpc": rpc_url,
        }));
    }

    println!(
        "▲ lattice v{} — post-quantum sharded ledger\n",
        env!("CARGO_PKG_VERSION")
    );
    println!(" status     •  {status}");
    println!(
        " peers      •  {} active [libp2p engine]",
        peers.len()
    );
    println!(" height     •  {} blocks [shard-mesh preview]", height);
    println!(" hashrate   •  local-only [argon2 pow]");
    println!(" storage    •  {:.2} GB [rocksdb local data]", storage_gb);
    if cli::output::verbose_enabled() {
        println!(" rpc        •  {}", rpc_url);
        println!(" wallet     •  {}", cli::wallet::get_default_wallet_path());
    }
    println!();
    println!(
        " [net] block #{} visible through rpc endpoint {}",
        height, rpc_url
    );
    println!();
    Ok(())
}

fn estimate_storage_gb() -> f64 {
    let data_dir = node::config::NodeConfig::default().data_dir;
    let bytes = dir_size_bytes(&data_dir);
    bytes as f64 / 1024.0 / 1024.0 / 1024.0
}

fn dir_size_bytes(path: &Path) -> u64 {
    if !path.exists() {
        return 0;
    }

    let mut total = 0u64;
    if let Ok(entries) = fs::read_dir(path) {
        for entry in entries.flatten() {
            let child = entry.path();
            if child.is_dir() {
                total = total.saturating_add(dir_size_bytes(&child));
            } else if let Ok(meta) = entry.metadata() {
                total = total.saturating_add(meta.len());
            }
        }
    }
    total
}

fn parse_lat_amount(s: &str) -> anyhow::Result<u128> {
    const DECIMALS: u32 = lattice_core::tokenomics::DECIMALS as u32;
    const MULTIPLIER: u128 = lattice_core::tokenomics::LATT_PER_LAT;

    if let Some(dot_pos) = s.find('.') {
        let whole_str = &s[..dot_pos];
        let frac_str = &s[dot_pos + 1..];
        if frac_str.contains('.') {
            anyhow::bail!("Invalid amount format: {s}");
        }

        let whole: u128 = if whole_str.is_empty() {
            0
        } else {
            whole_str
                .parse()
                .map_err(|_| anyhow::anyhow!("Invalid whole part: {whole_str}"))?
        };

        let frac_padded = if frac_str.len() >= DECIMALS as usize {
            frac_str[..DECIMALS as usize].to_string()
        } else {
            format!("{:0<width$}", frac_str, width = DECIMALS as usize)
        };

        let frac: u128 = frac_padded
            .parse()
            .map_err(|_| anyhow::anyhow!("Invalid fractional part: {frac_str}"))?;

        whole
            .checked_mul(MULTIPLIER)
            .and_then(|value| value.checked_add(frac))
            .ok_or_else(|| anyhow::anyhow!("Amount overflow"))
    } else {
        s.parse::<u128>()
            .map_err(|_| anyhow::anyhow!("Invalid amount: {s}"))?
            .checked_mul(MULTIPLIER)
            .ok_or_else(|| anyhow::anyhow!("Amount overflow"))
    }
}
