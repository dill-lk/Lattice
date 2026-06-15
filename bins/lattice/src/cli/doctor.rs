//! Local diagnostics helpers for the unified CLI.

use std::path::Path;

use anyhow::Result;
use colored::Colorize;
use dialoguer::{Confirm, Input, Select};
use lattice_wallet::Keystore;

use crate::node::config::{parse_network, MiningConfig, NodeConfig, P2pConfig, RpcConfig};

use crate::cli::formatter;
use crate::cli::output;
use crate::cli::rpc_client::RpcClient;

const DEFAULT_WALLET_FILE: &str = "wallet.json";

/// Run a simple operator diagnostics pass.
pub async fn run_doctor(rpc_url: &str) -> Result<()> {
    let wallet = check_wallet_result();
    let data_dir = check_data_dir_result();
    let rpc = check_rpc_result(rpc_url).await;

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": wallet.ok && data_dir.ok && rpc.ok,
            "action": "doctor",
            "wallet": {
                "ok": wallet.ok,
                "path": wallet.path,
                "address": wallet.address,
                "keystore_id": wallet.keystore_id,
                "error": wallet.error,
            },
            "data_dir": {
                "ok": data_dir.ok,
                "path": data_dir.path,
            },
            "rpc": {
                "ok": rpc.ok,
                "endpoint": rpc.endpoint,
                "height": rpc.height,
                "error": rpc.error,
            }
        }));
    }

    formatter::title("Lattice Doctor");
    formatter::divider();
    formatter::note(
        "This checks the current workspace wallet, RPC reachability, and data directory basics.",
    );
    if output::verbose_enabled() {
        formatter::note("Verbose mode is enabled, so setup hints are expanded.");
    }
    println!();

    print_wallet_result(&wallet);
    println!();
    print_data_dir_result(&data_dir);
    println!();
    print_rpc_result(&rpc);
    println!();

    formatter::subheader("Recommendations");
    formatter::note("If no wallet exists, run `lattice --wallet-new`.");
    formatter::note(
        "If RPC is offline, start a node with `lattice --node` or mine with `lattice --mine 4`.",
    );
    formatter::note("For advanced checks, use `lattice status`, `lattice peers`, and `lattice query block latest`.");
    if output::verbose_enabled() {
        formatter::note("If you prefer explicit control, use `lattice node ...` and `lattice miner ...` separately.");
    }
    println!();
    Ok(())
}

struct WalletCheckResult {
    ok: bool,
    path: String,
    address: Option<String>,
    keystore_id: Option<String>,
    error: Option<String>,
}

struct DataDirCheckResult {
    ok: bool,
    path: String,
}

struct RpcCheckResult {
    ok: bool,
    endpoint: String,
    height: Option<u64>,
    error: Option<String>,
}

fn check_wallet_result() -> WalletCheckResult {
    let wallet_path = Path::new(DEFAULT_WALLET_FILE);
    if !wallet_path.exists() {
        return WalletCheckResult {
            ok: false,
            path: DEFAULT_WALLET_FILE.to_string(),
            address: None,
            keystore_id: None,
            error: Some("default wallet not found".to_string()),
        };
    }

    match Keystore::load_from_file(wallet_path) {
        Ok(keystore) => WalletCheckResult {
            ok: true,
            path: DEFAULT_WALLET_FILE.to_string(),
            address: Some(keystore.address().to_string()),
            keystore_id: Some(keystore.id().to_string()),
            error: None,
        },
        Err(e) => WalletCheckResult {
            ok: false,
            path: DEFAULT_WALLET_FILE.to_string(),
            address: None,
            keystore_id: None,
            error: Some(e.to_string()),
        },
    }
}

fn print_wallet_result(result: &WalletCheckResult) {
    formatter::subheader("Wallet Check");
    if result.ok {
        formatter::key_value_colored("Default Wallet", "OK".green().bold());
        formatter::key_value("Path", &result.path);
        if let Some(address) = &result.address {
            formatter::key_value("Address", address);
        }
        if let Some(id) = &result.keystore_id {
            formatter::key_value("Keystore ID", id);
        }
    } else {
        formatter::key_value_colored("Default Wallet", "MISSING".yellow().bold());
        formatter::key_value("Expected Path", &result.path);
        if let Some(error) = &result.error {
            formatter::key_value("Error", error);
        }
    }
}

fn check_data_dir_result() -> DataDirCheckResult {
    let config = NodeConfig::default();
    let data_dir = config.data_dir;
    DataDirCheckResult {
        ok: data_dir.exists(),
        path: data_dir.to_string_lossy().to_string(),
    }
}

fn print_data_dir_result(result: &DataDirCheckResult) {
    formatter::subheader("Node Data Directory");
    if result.ok {
        formatter::key_value_colored("Data Directory", "OK".green().bold());
        formatter::key_value("Path", &result.path);
    } else {
        formatter::key_value_colored("Data Directory", "MISSING".yellow().bold());
        formatter::key_value("Expected Path", &result.path);
    }
}

async fn check_rpc_result(rpc_url: &str) -> RpcCheckResult {
    let client = RpcClient::new(rpc_url);
    match client.get_block_number().await {
        Ok(height) => RpcCheckResult {
            ok: true,
            endpoint: rpc_url.to_string(),
            height: Some(height),
            error: None,
        },
        Err(e) => RpcCheckResult {
            ok: false,
            endpoint: rpc_url.to_string(),
            height: None,
            error: Some(e.to_string()),
        },
    }
}

fn print_rpc_result(result: &RpcCheckResult) {
    formatter::subheader("RPC Connectivity");
    formatter::key_value("Endpoint", &result.endpoint);
    if result.ok {
        formatter::key_value_colored("RPC", "ONLINE".green().bold());
        if let Some(height) = result.height {
            formatter::key_value("Observed Height", &height.to_string());
        }
    } else {
        formatter::key_value_colored("RPC", "OFFLINE".red().bold());
        if let Some(error) = &result.error {
            formatter::key_value("Error", error);
        }
    }
}

pub fn run_config_wizard(path: &str) -> Result<()> {
    formatter::title("Node Config Wizard");
    formatter::divider();

    let network_items = ["mainnet", "testnet", "devnet"];
    let network_index = Select::new()
        .with_prompt("  Select network")
        .items(&network_items)
        .default(0)
        .interact()?;
    let network_name = network_items[network_index];

    let data_dir: String = Input::new()
        .with_prompt("  Data directory")
        .default(NodeConfig::default().data_dir.to_string_lossy().to_string())
        .interact_text()?;

    let rpc_host: String = Input::new()
        .with_prompt("  RPC host")
        .default("127.0.0.1".to_string())
        .interact_text()?;

    let rpc_port: u16 = Input::new()
        .with_prompt("  RPC port")
        .default(8545)
        .interact_text()?;

    let p2p_port: u16 = Input::new()
        .with_prompt("  P2P port")
        .default(30303)
        .interact_text()?;

    let enable_mining = Confirm::new()
        .with_prompt("  Enable built-in mining by default?")
        .default(false)
        .interact()?;

    let mining_threads: usize = if enable_mining {
        Input::new()
            .with_prompt("  Mining threads")
            .default(1usize)
            .interact_text()?
    } else {
        1
    };

    let coinbase: Option<String> = if enable_mining {
        let value: String = Input::new()
            .with_prompt("  Coinbase address")
            .allow_empty(true)
            .interact_text()?;
        if value.trim().is_empty() {
            None
        } else {
            Some(value)
        }
    } else {
        None
    };

    let mut p2p = P2pConfig::default();
    p2p.listen_addr.set_port(p2p_port);

    let config = NodeConfig {
        network: parse_network(network_name)?,
        data_dir: std::path::PathBuf::from(data_dir),
        rpc: RpcConfig {
            host: rpc_host,
            port: rpc_port,
            ..Default::default()
        },
        p2p,
        mining: MiningConfig {
            enabled: enable_mining,
            threads: mining_threads,
            coinbase,
            ..Default::default()
        },
        ..Default::default()
    };

    let output_path = std::path::PathBuf::from(path);
    if let Some(parent) = output_path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    config.save(&output_path)?;

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "config_init",
            "path": output_path.to_string_lossy(),
            "network": network_name,
            "rpc_host": config.rpc.host,
            "rpc_port": config.rpc.port,
            "p2p_port": config.p2p.listen_addr.port(),
            "mining_enabled": config.mining.enabled,
            "mining_threads": config.mining.threads,
        }));
    }

    formatter::success("Configuration written.");
    formatter::key_value("Path", &output_path.to_string_lossy());
    formatter::key_value("Network", network_name);
    formatter::key_value("RPC", &format!("{}:{}", config.rpc.host, config.rpc.port));
    formatter::key_value("P2P", &config.p2p.listen_addr.to_string());
    formatter::key_value(
        "Mining",
        if config.mining.enabled {
            "enabled"
        } else {
            "disabled"
        },
    );
    println!();
    Ok(())
}
