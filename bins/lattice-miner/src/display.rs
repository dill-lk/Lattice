//! Terminal display for the Lattice miner.
//!
//! Clean, minimal output - no visual clutter.

use crate::stats::{MiningStats, StatsSnapshot};
use colored::Colorize;
use std::io::{self, IsTerminal, Write};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::sync::mpsc;

const CLEAR_WIDTH: usize = 100;

// ── Events ────────────────────────────────────────────────────────────────────

/// Events that affect the miner display.
pub enum MinerEvent {
    /// New work template received from the node.
    WorkUpdate {
        height: u64,
        difficulty: u64,
        tx_count: usize,
    },
    /// A valid nonce was found (before RPC submission).
    BlockFound { height: u64, nonce: u64 },
    /// Node accepted our block submission.
    BlockAccepted { height: u64 },
    /// Node rejected our block submission.
    BlockRejected { height: u64 },
    /// Successfully connected (or reconnected) to the node.
    NodeConnected { height: u64 },
    /// Node RPC is unreachable.
    NodeError { attempt: u32 },
}

// ── One-shot prints ───────────────────────────────────────────────────────────

/// Print the startup banner - clean and minimal
pub fn print_banner(version: &str, threads: usize, coinbase: &str, rpc: &str, network: &str) {
    println!();
    println!(
        "  {}  {}",
        "LATTICE MINER".bold().cyan(),
        format!("v{}", version).dimmed()
    );
    println!("  {}", "─".repeat(50).dimmed());
    println!(
        "  {}  {}",
        "Network".dimmed(),
        network.to_uppercase().white()
    );
    println!("  {}  {}", "Threads".dimmed(), threads.to_string().white());
    println!("  {} {}", "Coinbase".dimmed(), coinbase.white());
    println!("  {}  {}", "RPC".dimmed(), rpc.dimmed());
    println!();
}

/// Print the session summary when the miner shuts down.
pub fn print_final_stats(stats: &MiningStats) {
    let snap = stats.snapshot();

    // Erase the live line first
    if io::stdout().is_terminal() {
        print!("\r{:<width$}\r", "", width = CLEAR_WIDTH);
        let _ = io::stdout().flush();
    }

    println!();
    println!("  {}", "Session Summary".bold());
    println!("  {}", "─".repeat(40).dimmed());
    println!("  Uptime       {}", stats.uptime_string().white());
    println!("  Hashes       {}", fmt_number(snap.total_hashes).cyan());
    println!(
        "  Avg Rate     {}",
        MiningStats::format_hash_rate(snap.average_hash_rate).green()
    );
    println!(
        "  Blocks       {} found  {} rejected",
        snap.blocks_found.to_string().green(),
        snap.blocks_rejected.to_string().red()
    );
    println!();
}

// ── Async display loop ────────────────────────────────────────────────────────

/// Long-running async task for display output.
pub async fn display_loop(
    mut event_rx: mpsc::Receiver<MinerEvent>,
    stats: Arc<MiningStats>,
    current_height: Arc<AtomicU64>,
    non_tty_stats_secs: u64,
) {
    let is_tty = io::stdout().is_terminal();
    let mut tick = tokio::time::interval(Duration::from_secs(1));
    tick.set_missed_tick_behavior(tokio::time::MissedTickBehavior::Skip);

    let mut elapsed_secs: u64 = 0;

    loop {
        tokio::select! {
            _ = tick.tick() => {
                elapsed_secs += 1;
                let h = current_height.load(Ordering::Relaxed);
                let snap = stats.snapshot();

                if is_tty {
                    print!("\r{}", live_line(&snap, h, &stats.uptime_string()));
                    let _ = io::stdout().flush();
                } else if elapsed_secs.is_multiple_of(non_tty_stats_secs) {
                    println!("{}", snap);
                }
            }

            event = event_rx.recv() => {
                match event {
                    None => break,
                    Some(ev) => handle_event(ev, &stats, &current_height, is_tty),
                }
            }
        }
    }
}

// ── Internal helpers ──────────────────────────────────────────────────────────

fn handle_event(
    ev: MinerEvent,
    stats: &MiningStats,
    current_height: &AtomicU64,
    is_tty: bool,
) {
    match ev {
        MinerEvent::WorkUpdate {
            height,
            difficulty,
            tx_count,
        } => {
            current_height.store(height, Ordering::Relaxed);
            let msg = format!(
                "  {} block {} · diff {} · {} tx",
                "→".blue(),
                height.to_string().bold(),
                fmt_difficulty(difficulty).dimmed(),
                tx_count,
            );
            print_above(&msg, is_tty);
        }

        MinerEvent::BlockFound { height, nonce } => {
            let _ = stats.snapshot();
            let msg = format!(
                "  {} block {} found · nonce {:x}",
                "●".yellow(),
                height.to_string().bold(),
                nonce,
            );
            print_above(&msg, is_tty);
        }

        MinerEvent::BlockAccepted { height } => {
            let msg = format!(
                "  {} block {} accepted · +10 LAT",
                "✓".green().bold(),
                height.to_string().green().bold(),
            );
            print_above(&msg, is_tty);
        }

        MinerEvent::BlockRejected { height } => {
            let msg = format!(
                "  {} block {} rejected",
                "✗".red(),
                height.to_string().red(),
            );
            print_above(&msg, is_tty);
        }

        MinerEvent::NodeConnected { height } => {
            let msg = format!(
                "  {} connected · block {}",
                "✓".green(),
                height.to_string().dimmed(),
            );
            print_above(&msg, is_tty);
        }

        MinerEvent::NodeError { attempt } => {
            if attempt <= 1 || attempt % 10 == 0 {
                let msg = format!(
                    "  {} node unreachable · attempt {}",
                    "!".yellow(),
                    attempt,
                );
                print_above(&msg, is_tty);
            }
        }
    }
}

fn print_above(msg: &str, is_tty: bool) {
    if is_tty {
        print!("\r{:<width$}\r{}\n", "", msg, width = CLEAR_WIDTH);
    } else {
        println!("{}", msg);
    }
    let _ = io::stdout().flush();
}

/// Build the live stats line - compact and readable
fn live_line(snap: &StatsSnapshot, height: u64, uptime: &str) -> String {
    let block = if height == 0 {
        "...".dimmed().to_string()
    } else {
        format!("#{}", height)
    };

    let rate = MiningStats::format_hash_rate(snap.current_hash_rate);
    let hashes = fmt_compact(snap.total_hashes);

    format!(
        "  {} {} {} {} {} {} {}",
        "mining".dimmed(),
        block.white(),
        "│".dimmed(),
        rate.green(),
        "│".dimmed(),
        hashes.cyan(),
        format!("│ {}", uptime).dimmed(),
    )
}

fn fmt_difficulty(d: u64) -> String {
    if d >= 1_000_000_000 {
        format!("{:.1}G", d as f64 / 1_000_000_000.0)
    } else if d >= 1_000_000 {
        format!("{:.1}M", d as f64 / 1_000_000.0)
    } else if d >= 1_000 {
        format!("{:.1}K", d as f64 / 1_000.0)
    } else {
        format!("{}", d)
    }
}

fn fmt_number(n: u64) -> String {
    let s = n.to_string();
    let mut out = String::with_capacity(s.len() + s.len() / 3);
    for (i, ch) in s.chars().rev().enumerate() {
        if i > 0 && i % 3 == 0 {
            out.push(',');
        }
        out.push(ch);
    }
    out.chars().rev().collect()
}

/// Compact number format (e.g., 1.2M instead of 1,234,567)
fn fmt_compact(n: u64) -> String {
    if n >= 1_000_000_000 {
        format!("{:.1}G", n as f64 / 1_000_000_000.0)
    } else if n >= 1_000_000 {
        format!("{:.1}M", n as f64 / 1_000_000.0)
    } else if n >= 1_000 {
        format!("{:.1}K", n as f64 / 1_000.0)
    } else {
        n.to_string()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt_difficulty() {
        assert_eq!(fmt_difficulty(500), "500");
        assert_eq!(fmt_difficulty(1_500), "1.5K");
        assert_eq!(fmt_difficulty(2_000_000), "2.0M");
    }

    #[test]
    fn test_fmt_number() {
        assert_eq!(fmt_number(0), "0");
        assert_eq!(fmt_number(1_234_567), "1,234,567");
    }

    #[test]
    fn test_fmt_compact() {
        assert_eq!(fmt_compact(500), "500");
        assert_eq!(fmt_compact(1_500), "1.5K");
        assert_eq!(fmt_compact(1_500_000), "1.5M");
    }
}
