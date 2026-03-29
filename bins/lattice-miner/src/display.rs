//! Terminal display for the Lattice miner.
//!
//! All user-visible output is routed through this module so that:
//!  - The startup banner and live stats reach **stdout** without log-level
//!    prefixes or timestamps.
//!  - Debug/trace logging (via `tracing`) goes to **stderr** and does not
//!    interleave with the live status line.
//!  - On a real terminal the stats line updates in-place (like xmrig).
//!  - When stdout is not a TTY (piped / redirected) events are printed as
//!    plain lines and the in-place update is skipped.

use crate::stats::{MiningStats, StatsSnapshot};
use colored::Colorize;
use std::io::{self, IsTerminal, Write};
use std::sync::{
    atomic::{AtomicU64, Ordering},
    Arc,
};
use std::time::Duration;
use tokio::sync::mpsc;

// ── Box geometry ──────────────────────────────────────────────────────────────
// Total line width: 2 border chars + BOX_INNER chars = BOX_INNER + 2
const BOX_INNER: usize = 59;
// Width used to "erase" the live stats line before printing an event
const CLEAR_WIDTH: usize = 120;

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

/// Print the startup banner directly to stdout (no tracing prefix).
pub fn print_banner(version: &str, threads: usize, coinbase: &str, rpc: &str) {
    let bar = "═".repeat(BOX_INNER);
    // Truncate long strings so they fit in the box without wrapping.
    let coinbase_disp = truncate(coinbase, BOX_INNER - 12);
    let rpc_disp = truncate(rpc, BOX_INNER - 12);

    let lines = [
        format!("╔{}╗", bar),
        format!(
            "║{:^inner$}║",
            format!("⛏  LATTICE MINER  v{}", version),
            inner = BOX_INNER
        ),
        format!("╠{}╣", bar),
        format!(
            "║  {:<width$}║",
            format!("Threads : {}", threads),
            width = BOX_INNER - 2
        ),
        format!(
            "║  {:<width$}║",
            format!("Coinbase: {}", coinbase_disp),
            width = BOX_INNER - 2
        ),
        format!(
            "║  {:<width$}║",
            format!("Node RPC: {}", rpc_disp),
            width = BOX_INNER - 2
        ),
        format!("╚{}╝", bar),
    ];

    println!();
    for line in &lines {
        println!("{}", line.cyan().bold());
    }
    println!();
}

/// Print the session summary to stdout when the miner shuts down.
pub fn print_final_stats(stats: &MiningStats) {
    let snap = stats.snapshot();
    let bar = "═".repeat(BOX_INNER);

    // Erase the live line first (cursor may be mid-line).
    if io::stdout().is_terminal() {
        print!("\r{:<width$}\r\n", "", width = CLEAR_WIDTH);
        let _ = io::stdout().flush();
    }

    let lines = [
        format!("╔{}╗", bar),
        format!("║{:^inner$}║", "Session Summary", inner = BOX_INNER),
        format!("╠{}╣", bar),
        format!(
            "║  {:<width$}║",
            format!("Uptime      : {}", stats.uptime_string()),
            width = BOX_INNER - 2
        ),
        format!(
            "║  {:<width$}║",
            format!("Total hashes: {}", fmt_number(snap.total_hashes)),
            width = BOX_INNER - 2
        ),
        format!(
            "║  {:<width$}║",
            format!(
                "Avg hashrate: {}",
                MiningStats::format_hash_rate(snap.average_hash_rate)
            ),
            width = BOX_INNER - 2
        ),
        format!(
            "║  {:<width$}║",
            format!(
                "Blocks found: {}  rejected: {}",
                snap.blocks_found, snap.blocks_rejected
            ),
            width = BOX_INNER - 2
        ),
        format!("╚{}╝", bar),
    ];

    println!();
    for line in &lines {
        println!("{}", line.cyan());
    }
    println!();
}

// ── Async display loop ────────────────────────────────────────────────────────

/// Long-running async task that owns all stdout output after startup.
///
/// * Receives [`MinerEvent`]s from worker tasks and prints them cleanly.
/// * On a TTY: refreshes a live stats line in-place every second.
/// * Not a TTY: prints important events as plain lines; stats every
///   `non_tty_stats_secs` seconds.
pub async fn display_loop(
    mut event_rx: mpsc::Receiver<MinerEvent>,
    stats: Arc<MiningStats>,
    current_height: Arc<AtomicU64>,
    non_tty_stats_secs: u64,
) {
    let is_tty = io::stdout().is_terminal();

    // One-second tick drives the live stats line.
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
                    let line = live_line(&snap, h, &stats.uptime_string());
                    print!("\r{}", line);
                    let _ = io::stdout().flush();
                } else if elapsed_secs % non_tty_stats_secs == 0 {
                    // Non-TTY fallback: plain stats line periodically.
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
        MinerEvent::WorkUpdate { height, difficulty, tx_count } => {
            current_height.store(height, Ordering::Relaxed);
            let msg = format!(
                " {}  Block {}  ·  difficulty {}  ·  {} tx",
                "→".blue().bold(),
                format!("#{}", height).white().bold(),
                fmt_difficulty(difficulty).dimmed(),
                tx_count,
            );
            print_above(&msg, is_tty);
        }

        MinerEvent::BlockFound { height, nonce } => {
            // Refresh hashrate before the congratulatory line.
            let _ = stats.snapshot();
            let msg = format!(
                " 🎉  Block {}  found!  nonce=0x{:x}  · submitting...",
                format!("#{}", height).yellow().bold(),
                nonce,
            );
            print_above(&msg, is_tty);
        }

        MinerEvent::BlockAccepted { height } => {
            let msg = format!(
                " {}  Block {}  accepted  (+10 LAT)",
                "✓".green().bold(),
                format!("#{}", height).green().bold(),
            );
            print_above(&msg, is_tty);
        }

        MinerEvent::BlockRejected { height } => {
            let msg = format!(
                " {}  Block {}  rejected  (stale or invalid)",
                "✗".red().bold(),
                format!("#{}", height).red(),
            );
            print_above(&msg, is_tty);
        }

        MinerEvent::NodeConnected { height } => {
            let msg = format!(
                " {}  Connected to node  ·  current block {}",
                "✓".green().bold(),
                format!("#{}", height).white(),
            );
            print_above(&msg, is_tty);
        }

        MinerEvent::NodeError { attempt } => {
            // Only print on the first attempt and every 10th thereafter
            // to avoid flooding the output with repeated errors.
            if attempt <= 1 || attempt % 10 == 0 {
                let msg = format!(
                    " {}  Node unreachable (attempt {})  · retrying...",
                    "⚠".yellow().bold(),
                    attempt,
                );
                print_above(&msg, is_tty);
            }
        }
    }
}

/// Print `msg` above the live stats line.
///
/// On a TTY:
///   1. Move to start of the stats line (`\r`).
///   2. Overwrite with spaces to erase it.
///   3. Return to start (`\r`) and print the message followed by a newline.
///
/// Not a TTY: just `println!`.
fn print_above(msg: &str, is_tty: bool) {
    if is_tty {
        print!("\r{:<width$}\r{}\n", "", msg, width = CLEAR_WIDTH);
    } else {
        println!("{}", msg);
    }
    let _ = io::stdout().flush();
}

/// Build the live stats line (no trailing newline — updated with `\r`).
fn live_line(snap: &StatsSnapshot, height: u64, uptime: &str) -> String {
    let block = if height == 0 {
        "waiting for work...".dimmed().to_string()
    } else {
        format!("#{}", height).white().bold().to_string()
    };

    let rate = MiningStats::format_hash_rate(snap.current_hash_rate);
    let avg = MiningStats::format_hash_rate(snap.average_hash_rate);
    let found = snap.blocks_found.to_string();

    format!(
        " {}  {}  │  {}  (avg {})  │  {} found  │  up {}   ",
        "⛏",
        block,
        rate.green(),
        avg.dimmed(),
        found.yellow(),
        uptime,
    )
}

/// Format a difficulty value as a human-readable string (e.g. `1.0M`).
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

/// Format a large integer with thousands separators (e.g. `1,234,567`).
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

/// Truncate a string to `max` chars (character-aware), appending `...` if truncated.
fn truncate(s: &str, max: usize) -> String {
    let chars: Vec<char> = s.chars().collect();
    if chars.len() <= max {
        s.to_owned()
    } else {
        let truncated: String = chars[..max.saturating_sub(3)].iter().collect();
        format!("{}...", truncated)
    }
}

// ── Tests ─────────────────────────────────────────────────────────────────────

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_fmt_difficulty() {
        assert_eq!(fmt_difficulty(500), "500");
        assert_eq!(fmt_difficulty(1_500), "1.5K");
        assert_eq!(fmt_difficulty(2_000_000), "2.0M");
        assert_eq!(fmt_difficulty(3_000_000_000), "3.0G");
    }

    #[test]
    fn test_fmt_number() {
        assert_eq!(fmt_number(0), "0");
        assert_eq!(fmt_number(999), "999");
        assert_eq!(fmt_number(1_000), "1,000");
        assert_eq!(fmt_number(1_234_567), "1,234,567");
    }

    #[test]
    fn test_truncate() {
        assert_eq!(truncate("hello", 10), "hello");
        assert_eq!(truncate("hello world", 8), "hello...");
        // Multi-byte characters must not cause a panic
        assert_eq!(truncate("こんにちは世界", 5), "こん...");
    }
}
