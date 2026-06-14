//! Node-related command handlers.

use anyhow::Result;
use colored::Colorize;

use crate::cli::formatter;
use crate::cli::output;
use crate::cli::rpc_client::RpcClient;

/// Display node sync status.
pub async fn show_status(rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    formatter::title("Node Status");
    formatter::divider();
    formatter::key_value("RPC Endpoint", rpc_url);

    match client.get_sync_status().await {
        Ok(status) => {
            let progress = if status.highest_block > 0 {
                (status.current_block as f64 / status.highest_block as f64) * 100.0
            } else {
                100.0
            };
            let blocks_behind = status.highest_block.saturating_sub(status.current_block);

            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": true,
                    "action": "node_status",
                    "rpc": rpc_url,
                    "syncing": status.syncing,
                    "current_block": status.current_block,
                    "highest_block": status.highest_block,
                    "progress_percent": progress,
                    "blocks_behind": blocks_behind,
                }));
            }

            if status.syncing {
                formatter::key_value_colored("State", "SYNCING".yellow().bold());
                formatter::key_value("Current Block", &status.current_block.to_string());
                formatter::key_value("Highest Block", &status.highest_block.to_string());
                formatter::key_value("Progress", &format!("{progress:.2}%"));
                formatter::key_value("Blocks Behind", &blocks_behind.to_string());
                formatter::note("Sync reporting currently comes from RPC and may be conservative.");
            } else {
                formatter::key_value_colored("State", "SYNCED".green().bold());
                formatter::key_value("Block Height", &status.current_block.to_string());
            }
        }
        Err(e) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "node_status",
                    "rpc": rpc_url,
                    "error": e.to_string(),
                }));
            }
            formatter::key_value_colored("State", "OFFLINE".red().bold());
            formatter::error(&format!("Cannot connect to node: {e}"));
            formatter::note("Make sure a Lattice node is running and the RPC endpoint is reachable.");
        }
    }

    println!();
    Ok(())
}

pub async fn show_chain(rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);
    let latest = client.get_latest_block(true).await?;
    let height = client.get_block_number().await.unwrap_or(0);
    let difficulty = latest.get("difficulty").and_then(|v| v.as_str()).unwrap_or("0x0");
    let tx_count = latest
        .get("transactions")
        .and_then(|v| v.as_array())
        .map(|items| items.len())
        .unwrap_or(0);
    let hash = latest.get("hash").and_then(|v| v.as_str()).unwrap_or("0x0");
    let parent = latest.get("parentHash").and_then(|v| v.as_str()).unwrap_or("0x0");

    if output::json_enabled() {
        return output::emit_json(serde_json::json!({
            "ok": true,
            "action": "chain_summary",
            "rpc": rpc_url,
            "height": height,
            "difficulty": difficulty,
            "hash": hash,
            "parent_hash": parent,
            "tx_count": tx_count,
            "hashrate": null,
        }));
    }

    formatter::title("Chain Summary");
    formatter::divider();
    formatter::key_value("RPC Endpoint", rpc_url);
    formatter::key_value("Height", &height.to_string());
    formatter::key_value("Difficulty", difficulty);
    formatter::key_value("Latest Hash", hash);
    formatter::key_value("Parent Hash", parent);
    formatter::key_value("Transactions", &tx_count.to_string());
    formatter::key_value("Hashrate", "not exposed yet via RPC");
    if output::verbose_enabled() {
        formatter::note("Hashrate will become more meaningful once network telemetry is exposed in the node RPC layer.");
    }
    println!();
    Ok(())
}

pub async fn show_mempool(rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);
    match client.get_mempool_stats().await {
        Ok(stats) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": true,
                    "action": "mempool_status",
                    "rpc": rpc_url,
                    "pending_count": stats.pending_count,
                    "pending_hashes": stats.pending_hashes,
                }));
            }

            formatter::title("Mempool Status");
            formatter::divider();
            formatter::key_value("RPC Endpoint", rpc_url);
            formatter::key_value("Pending Tx Count", &stats.pending_count.to_string());
            if output::verbose_enabled() && !stats.pending_hashes.is_empty() {
                formatter::subheader("Pending Hashes");
                for hash in stats.pending_hashes {
                    println!("  {}", hash);
                }
            }
            println!();
            Ok(())
        }
        Err(e) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "mempool_status",
                    "rpc": rpc_url,
                    "error": e.to_string(),
                }));
            }
            formatter::title("Mempool Status");
            formatter::divider();
            formatter::error(&format!("Cannot fetch mempool status: {e}"));
            println!();
            Ok(())
        }
    }
}

/// List connected peers.
pub async fn list_peers(rpc_url: &str) -> Result<()> {
    let client = RpcClient::new(rpc_url);

    formatter::title("Peer Overview");
    formatter::divider();
    formatter::key_value("RPC Endpoint", rpc_url);

    match client.get_peers().await {
        Ok(peers) => {
            if output::json_enabled() {
                let rows = peers
                    .iter()
                    .map(|peer| serde_json::json!({
                        "id": peer.id,
                        "address": peer.address,
                        "latency_ms": peer.latency_ms,
                        "score": peer.score,
                    }))
                    .collect::<Vec<_>>();
                return output::emit_json(serde_json::json!({
                    "ok": true,
                    "action": "node_peers",
                    "rpc": rpc_url,
                    "count": rows.len(),
                    "peers": rows,
                }));
            }

            if peers.is_empty() {
                formatter::warning("No peers reported by this node.");
                formatter::note("This may be expected while networking is still starting up.");
                formatter::note("Some peer RPC fields are still limited until full P2P integration lands.");
            } else {
                formatter::key_value("Peer Count", &peers.len().to_string());
                println!();
                formatter::table_header(&["Peer ID", "Address", "Latency", "Score"]);
                for peer in peers {
                    formatter::table_row(&[
                        peer.id,
                        peer.address,
                        format!("{} ms", peer.latency_ms),
                        peer.score.to_string(),
                    ]);
                }
            }
        }
        Err(e) => {
            if output::json_enabled() {
                return output::emit_json(serde_json::json!({
                    "ok": false,
                    "action": "node_peers",
                    "rpc": rpc_url,
                    "error": e.to_string(),
                }));
            }
            formatter::error(&format!("Cannot fetch peer information: {e}"));
        }
    }

    println!();
    Ok(())
}
