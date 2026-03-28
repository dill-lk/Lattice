//! Node command handlers

use anyhow::Result;

use crate::rpc_client::RpcClient;

/// Display node sync status
pub async fn show_status(rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    println!("Node Status");
    println!("===========");
    println!();
    println!("RPC Endpoint: {}", rpc_url);
    println!();

    match client.get_sync_status().await {
        Ok(status) => {
            if status.syncing {
                println!("Sync Status: Syncing");
                println!("Current Block: {}", status.current_block);
                println!("Highest Block: {}", status.highest_block);

                let progress = if status.highest_block > 0 {
                    (status.current_block as f64 / status.highest_block as f64) * 100.0
                } else {
                    100.0
                };
                println!("Progress: {:.2}%", progress);

                let blocks_behind = status.highest_block.saturating_sub(status.current_block);
                println!("Blocks Behind: {}", blocks_behind);
            } else {
                println!("Sync Status: ✓ Synced");
                println!("Block Height: {}", status.current_block);
            }
        }
        Err(e) => {
            println!("Status: ✗ Cannot connect to node");
            println!();
            println!("Error: {}", e);
            println!();
            println!("Make sure the Lattice node is running and accessible at:");
            println!("  {}", rpc_url);
        }
    }

    Ok(())
}

/// List connected peers
pub async fn list_peers(rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    println!("Connected Peers");
    println!("===============");
    println!();

    match client.get_peers().await {
        Ok(peers) => {
            if peers.is_empty() {
                println!("No peers connected");
                println!();
                println!("This could mean:");
                println!("  - The node is still discovering peers");
                println!("  - Network connectivity issues");
                println!("  - No other nodes are available on the network");
            } else {
                println!("Total: {} peer(s)", peers.len());
                println!();

                for (i, peer) in peers.iter().enumerate() {
                    println!("[{}] {}", i + 1, peer.id);
                    println!("    Address: {}", peer.address);
                    println!("    Latency: {} ms", peer.latency_ms);
                }
            }
        }
        Err(e) => {
            println!("Error: Cannot fetch peer information");
            println!();
            println!("Details: {}", e);
            println!();
            println!("Make sure the Lattice node is running at: {}", rpc_url);
        }
    }

    Ok(())
}
