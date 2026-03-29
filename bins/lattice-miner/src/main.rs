//! Lattice Miner - Standalone mining application
//!
//! A multi-threaded CPU miner for the Lattice blockchain.
//! Connects to a node via JSON-RPC to fetch work and submit solutions.

mod display;
mod rpc_client;
mod stats;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use display::MinerEvent;
use lattice_consensus::{Miner, MiningResult, PoWConfig};
use lattice_core::Address;
use parking_lot::RwLock;
use rpc_client::{RpcClient, WorkSolution, WorkTemplate};
use stats::MiningStats;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tokio::sync::mpsc;
use tracing::{debug, error, info, warn};
use tracing_subscriber::EnvFilter;

/// Command line arguments
#[derive(Parser)]
#[command(name = "lattice-miner")]
#[command(about = "Lattice blockchain miner")]
#[command(version)]
struct Args {
    /// Number of mining threads (0 = auto-detect CPU cores)
    #[arg(long, short, default_value = "0")]
    threads: usize,

    /// Coinbase address for block rewards
    #[arg(long, short)]
    coinbase: String,

    /// RPC endpoint URL to connect to
    #[arg(long, short, default_value = "http://127.0.0.1:8545")]
    rpc: String,

    /// Work polling interval in milliseconds
    #[arg(long, default_value = "1000")]
    poll_interval: u64,

    /// Statistics display interval in seconds (non-TTY mode only)
    #[arg(long, default_value = "10")]
    stats_interval: u64,

    /// Network to mine on (determines PoW difficulty)
    #[arg(long, default_value = "mainnet")]
    network: String,

    /// Use light PoW config (overrides --network setting)
    #[arg(long)]
    light: bool,
}

/// Message sent when a solution is found
#[derive(Debug)]
struct SolutionFound {
    work_id: String,
    nonce: u64,
    pow_hash: [u8; 32],
    /// Block height at which the solution was found (for display)
    height: u64,
}

/// Shared state for the mining coordinator
struct MinerState {
    /// Current work template
    current_work: RwLock<Option<WorkTemplate>>,
    /// Flag to signal work update
    work_updated: AtomicBool,
    /// Flag to signal shutdown
    shutdown: AtomicBool,
}

impl MinerState {
    fn new() -> Arc<Self> {
        Arc::new(Self {
            current_work: RwLock::new(None),
            work_updated: AtomicBool::new(false),
            shutdown: AtomicBool::new(false),
        })
    }
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    // Route tracing logs to stderr so they don't interleave with the live
    // stats line that display_loop writes to stdout.
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("lattice_miner=info".parse().unwrap())
                .add_directive("lattice_consensus=debug".parse().unwrap()),
        )
        .with_writer(std::io::stderr)
        .init();

    // Validate coinbase address before printing the banner.
    let coinbase = Address::from_base58(&args.coinbase)
        .map_err(|e| anyhow!("Invalid coinbase address: {}", e))?;

    let num_threads = if args.threads == 0 {
        num_cpus::get()
    } else {
        args.threads
    };

    // Print the startup banner directly to stdout (no log prefix / timestamp).
    display::print_banner(
        env!("CARGO_PKG_VERSION"),
        num_threads,
        &args.coinbase,
        &args.rpc,
        &args.network,
    );

    // Display event channel — all worker tasks send events here; the display
    // loop owns stdout and serialises all output.
    let (event_tx, event_rx) = mpsc::channel::<MinerEvent>(64);

    // Shared current block height used by the live stats line.
    let current_height = Arc::new(AtomicU64::new(0));

    let rpc_client = Arc::new(RpcClient::new(&args.rpc).context("Failed to create RPC client")?);
    let stats = MiningStats::new();
    let state = MinerState::new();

    // Create solution channel
    let (solution_tx, mut solution_rx) = mpsc::channel::<SolutionFound>(32);

    // PoW configuration based on network
    let pow_config = if args.light {
        warn!("Using light PoW config (testing only!)");
        PoWConfig::light()
    } else {
        match args.network.to_lowercase().as_str() {
            "devnet" | "dev" => {
                info!("Using devnet PoW config (fast blocks, ~2 sec)");
                PoWConfig::devnet()
            }
            "testnet" | "test" => {
                info!("Using testnet PoW config (moderate speed, ~5 sec)");
                PoWConfig::testnet()
            }
            _ => {
                info!("Using mainnet PoW config (full security, ~15 sec blocks)");
                PoWConfig::mainnet()
            }
        }
    };

    // ── Spawn tasks ──────────────────────────────────────────────────────────

    // Work polling: fetches new block templates from the node.
    let poll_state = Arc::clone(&state);
    let poll_rpc = Arc::clone(&rpc_client);
    let poll_coinbase = coinbase.clone();
    let poll_interval = Duration::from_millis(args.poll_interval);
    let poll_event_tx = event_tx.clone();

    let poll_handle = tokio::spawn(async move {
        work_polling_loop(poll_state, poll_rpc, poll_coinbase, poll_interval, poll_event_tx).await
    });

    // Mining threads: CPU-intensive, run on a dedicated OS thread.
    let mining_state = Arc::clone(&state);
    let mining_stats = Arc::clone(&stats);
    let mining_handle = std::thread::spawn(move || {
        mining_loop(
            mining_state,
            mining_stats,
            num_threads,
            pow_config,
            solution_tx,
        )
    });

    // Solution submission: takes found nonces and submits them to the node.
    let submit_rpc = Arc::clone(&rpc_client);
    let submit_stats = Arc::clone(&stats);
    let submit_event_tx = event_tx.clone();

    let submit_handle = tokio::spawn(async move {
        while let Some(solution) = solution_rx.recv().await {
            handle_solution(&submit_rpc, &submit_stats, solution, &submit_event_tx).await;
        }
    });

    // Display loop: owns all stdout output after the banner.
    let display_stats = Arc::clone(&stats);
    let display_height = Arc::clone(&current_height);
    let display_handle = tokio::spawn(display::display_loop(
        event_rx,
        display_stats,
        display_height,
        args.stats_interval,
    ));

    // ── Wait for Ctrl+C ───────────────────────────────────────────────────────
    info!("Miner started. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;

    // ── Graceful shutdown ────────────────────────────────────────────────────
    info!("Shutting down miner…");
    state.shutdown.store(true, Ordering::SeqCst);

    poll_handle.abort();
    submit_handle.abort();
    // Drop event_tx so the display loop's receiver closes and the task ends.
    drop(event_tx);
    let _ = display_handle.await;
    let _ = mining_handle.join();

    // Print final session summary directly to stdout.
    display::print_final_stats(&stats);

    info!("Miner stopped.");
    Ok(())
}

// ── Work polling ─────────────────────────────────────────────────────────────

/// Poll the node for new work and update the shared work template.
async fn work_polling_loop(
    state: Arc<MinerState>,
    rpc: Arc<RpcClient>,
    coinbase: Address,
    interval: Duration,
    event_tx: mpsc::Sender<MinerEvent>,
) {
    let mut consecutive_errors = 0u32;
    let mut connected = false;

    while !state.shutdown.load(Ordering::Relaxed) {
        match rpc.get_work(&coinbase).await {
            Ok(work) => {
                if !connected || consecutive_errors > 0 {
                    connected = true;
                    consecutive_errors = 0;
                    let height = work.header.height;
                    let _ = event_tx.send(MinerEvent::NodeConnected { height }).await;
                }

                let new_height = work.header.height;
                let new_difficulty = work.header.difficulty;
                let tx_count = work.tx_count;

                let should_update = {
                    let current = state.current_work.read();
                    match &*current {
                        Some(existing) => {
                            existing.header.height != new_height
                                || existing.header.prev_hash != work.header.prev_hash
                        }
                        None => true,
                    }
                };

                if should_update {
                    debug!(
                        "New work: height={}, difficulty={}, txs={}",
                        new_height, new_difficulty, tx_count
                    );
                    *state.current_work.write() = Some(work);
                    state.work_updated.store(true, Ordering::SeqCst);

                    let _ = event_tx
                        .send(MinerEvent::WorkUpdate {
                            height: new_height,
                            difficulty: new_difficulty,
                            tx_count,
                        })
                        .await;
                }
            }
            Err(e) => {
                consecutive_errors += 1;
                let backoff = Duration::from_millis(
                    interval.as_millis() as u64 * consecutive_errors.min(10) as u64,
                );

                debug!("Failed to get work: {} (retry in {:?})", e, backoff);
                let _ = event_tx
                    .send(MinerEvent::NodeError {
                        attempt: consecutive_errors,
                    })
                    .await;

                tokio::time::sleep(backoff).await;
                continue;
            }
        }

        tokio::time::sleep(interval).await;
    }
}

// ── Mining loop ───────────────────────────────────────────────────────────────

/// Main mining loop — runs on a dedicated OS thread.
fn mining_loop(
    state: Arc<MinerState>,
    stats: Arc<MiningStats>,
    num_threads: usize,
    pow_config: PoWConfig,
    solution_tx: mpsc::Sender<SolutionFound>,
) {
    let miner = Miner::new(pow_config).with_threads(num_threads);

    while !state.shutdown.load(Ordering::Relaxed) {
        // Wait until work is available.
        let work = loop {
            if state.shutdown.load(Ordering::Relaxed) {
                return;
            }
            if let Some(work) = state.current_work.read().clone() {
                state.work_updated.store(false, Ordering::SeqCst);
                break work;
            }
            std::thread::sleep(Duration::from_millis(100));
        };

        debug!("Mining block at height {}", work.header.height);

        // With Argon2 memory-hard PoW, each hash takes ~1-2 seconds
        // Use small chunks so stats update frequently
        let chunk_size: u64 = 50;  // Small chunks for responsive stats
        let mut start_nonce: u64 = 0;
        let mut last_stats_update = Instant::now();

        loop {
            if state.shutdown.load(Ordering::Relaxed) {
                return;
            }
            if state.work_updated.load(Ordering::Relaxed) {
                debug!("Work updated, restarting");
                break;
            }

            let end_nonce = start_nonce.saturating_add(chunk_size);

            match miner.mine_range(&work.header, start_nonce, end_nonce) {
                Ok(MiningResult::Found { nonce, hash }) => {
                    info!("Found valid nonce: {} for height {}", nonce, work.header.height);

                    let solution = SolutionFound {
                        work_id: work.work_id.clone(),
                        nonce,
                        pow_hash: hash,
                        height: work.header.height,
                    };

                    if let Err(e) = solution_tx.blocking_send(solution) {
                        error!("Failed to send solution: {}", e);
                    }
                    break;
                }
                Ok(MiningResult::Exhausted) => {
                    start_nonce = end_nonce;
                    if start_nonce == u64::MAX {
                        debug!("Nonce space exhausted, waiting for new work");
                        break;
                    }
                }
                Ok(MiningResult::Cancelled) => {
                    debug!("Mining cancelled");
                    break;
                }
                Err(e) => {
                    error!("Mining error: {}", e);
                    std::thread::sleep(Duration::from_secs(1));
                    break;
                }
            }

            // Update stats after each chunk for responsive display
            let miner_stats = miner.stats();
            if miner_stats.hashes > 0 {
                stats.add_hashes(miner_stats.hashes);
            }
            
            // Log progress every 5 seconds
            if last_stats_update.elapsed() >= Duration::from_secs(5) {
                let snap = stats.snapshot();
                debug!(
                    "Mining progress: {} hashes, {:.2} H/s",
                    snap.total_hashes,
                    snap.current_hash_rate
                );
                last_stats_update = Instant::now();
            }
        }
    }
}

// ── Solution submission ───────────────────────────────────────────────────────

/// Submit a found solution to the node and report the outcome.
async fn handle_solution(
    rpc: &RpcClient,
    stats: &MiningStats,
    solution: SolutionFound,
    event_tx: &mpsc::Sender<MinerEvent>,
) {
    let height = solution.height;
    let nonce = solution.nonce;

    // Notify the display loop that we found a nonce (before submitting).
    let _ = event_tx
        .send(MinerEvent::BlockFound { height, nonce })
        .await;

    let work_solution = WorkSolution {
        work_id: solution.work_id,
        nonce,
        pow_hash: format!("0x{}", hex::encode(solution.pow_hash)),
    };

    match rpc.submit_work(&work_solution).await {
        Ok(true) => {
            info!("Block {} accepted by node", height);
            stats.record_block_found();
            let _ = event_tx
                .send(MinerEvent::BlockAccepted { height })
                .await;
        }
        Ok(false) => {
            warn!("Block {} rejected by node (stale or invalid)", height);
            stats.record_block_rejected();
            let _ = event_tx
                .send(MinerEvent::BlockRejected { height })
                .await;
        }
        Err(e) => {
            error!("Failed to submit solution for height {}: {}", height, e);
            stats.record_block_rejected();
            let _ = event_tx
                .send(MinerEvent::BlockRejected { height })
                .await;
        }
    }
}

// ── CPU count helper ──────────────────────────────────────────────────────────

mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1)
    }
}

