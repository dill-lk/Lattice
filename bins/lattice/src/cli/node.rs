//! Node command handlers

use anyhow::Result;

use crate::cli::rpc_client::RpcClient;
use crate::cli::formatter;

/// Display node sync status
pub async fn show_status(rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    formatter::header("Node Sync Status");
    formatter::key_value("RPC Endpoint", rpc_url);

    match client.get_sync_status().await {
        Ok(status) => {
            if status.syncing {
                formatter::key_value("Status", "Syncing ⏳");
                formatter::key_value("Current Block", &status.current_block.to_string());
                formatter::key_value("Highest Block", &status.highest_block.to_string());

                let progress = if status.highest_block > 0 {
                    (status.current_block as f64 / status.highest_block as f64) * 100.0
                } else {
                    100.0
                };
                formatter::key_value("Progress", &format!("{:.2}%", progress));

                let blocks_behind = status.highest_block.saturating_sub(status.current_block);
                formatter::key_value("Blocks Behind", &blocks_behind.to_string());
            } else {
                formatter::key_value_colored("Status", "Synced".green().bold());
                formatter::key_value("Block Height", &status.current_block.to_string());
            }
        }
        Err(e) => {
            formatter::key_value_colored("Status", "Offline".red().bold());
            println!();
            formatter::error(&format!("Cannot connect to node: {}", e));
            println!("  Make sure the Lattice node is running and accessible.");
        }
    }
    println!();

    Ok(())
}

/// List connected peers
pub async fn list_peers(rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    formatter::header("Connected Network Peers");

    match client.get_peers().await {
        Ok(peers) => {
            if peers.is_empty() {
                formatter::warning("No peers connected directly to this node.");
                println!("  This could mean peer discovery is still in progress or no other nodes are active.");
            } else {
                formatter::key_value("Total Peers", &peers.len().to_string());
                println!();

                for (i, peer) in peers.iter().enumerate() {
                    println!("  [{}] {}", i + 1, peer.id);
                    println!("      Address: {}", peer.address);
                    println!("      Latency: {} ms", peer.latency_ms);
                }
            }
        }
        Err(e) => {
            formatter::error(&format!("Cannot fetch peer information: {}", e));
        }
    }
    println!();

    Ok(())
}
