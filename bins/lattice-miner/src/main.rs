//! Lattice Miner - Standalone mining application
//!
//! A multi-threaded CPU miner for the Lattice blockchain.
//! Connects to a node via JSON-RPC to fetch work and submit solutions.

mod rpc_client;
mod stats;

use anyhow::{anyhow, Context, Result};
use clap::Parser;
use lattice_consensus::{Miner, MiningResult, PoWConfig};
use lattice_core::Address;
use parking_lot::RwLock;
use rpc_client::{RpcClient, WorkSolution, WorkTemplate};
use stats::MiningStats;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Arc;
use std::time::Duration;
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

    /// Statistics display interval in seconds
    #[arg(long, default_value = "10")]
    stats_interval: u64,

    /// Use light PoW config (for testing only)
    #[arg(long, hide = true)]
    light: bool,
}

/// Message sent when a solution is found
#[derive(Debug)]
struct SolutionFound {
    work_id: String,
    nonce: u64,
    pow_hash: [u8; 32],
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
    // Initialize logging
    tracing_subscriber::fmt()
        .with_env_filter(
            EnvFilter::from_default_env()
                .add_directive("lattice_miner=info".parse().unwrap())
                .add_directive("lattice_consensus=debug".parse().unwrap()),
        )
        .init();

    let args = Args::parse();

    // Parse coinbase address
    let coinbase = Address::from_base58(&args.coinbase)
        .map_err(|e| anyhow!("Invalid coinbase address: {}", e))?;

    // Determine thread count
    let num_threads = if args.threads == 0 {
        num_cpus::get()
    } else {
        args.threads
    };

    info!("╔═══════════════════════════════════════════╗");
    info!("║         Lattice Miner v{}          ║", env!("CARGO_PKG_VERSION"));
    info!("╠═══════════════════════════════════════════╣");
    info!("║  Threads:  {:<30} ║", num_threads);
    info!("║  Coinbase: {}...  ║", &args.coinbase[..20.min(args.coinbase.len())]);
    info!("║  RPC:      {:<30} ║", &args.rpc[..30.min(args.rpc.len())]);
    info!("╚═══════════════════════════════════════════╝");

    // Initialize components
    let rpc_client = Arc::new(RpcClient::new(&args.rpc).context("Failed to create RPC client")?);
    let stats = MiningStats::new();
    let state = MinerState::new();

    // Test RPC connection
    info!("Connecting to node at {}...", args.rpc);
    match rpc_client.block_number().await {
        Ok(height) => info!("Connected! Current block height: {}", height),
        Err(e) => {
            warn!("Could not connect to node: {}. Will retry...", e);
        }
    }

    // Create solution channel
    let (solution_tx, mut solution_rx) = mpsc::channel::<SolutionFound>(32);

    // PoW configuration
    let pow_config = if args.light {
        warn!("Using light PoW config (testing only!)");
        PoWConfig::light()
    } else {
        PoWConfig::default()
    };

    // Spawn work polling task
    let poll_state = Arc::clone(&state);
    let poll_rpc = Arc::clone(&rpc_client);
    let poll_coinbase = coinbase.clone();
    let poll_interval = Duration::from_millis(args.poll_interval);
    
    let poll_handle = tokio::spawn(async move {
        work_polling_loop(poll_state, poll_rpc, poll_coinbase, poll_interval).await
    });

    // Spawn mining threads
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

    // Spawn solution submission task
    let submit_rpc = Arc::clone(&rpc_client);
    let submit_stats = Arc::clone(&stats);
    
    let submit_handle = tokio::spawn(async move {
        while let Some(solution) = solution_rx.recv().await {
            handle_solution(
                &submit_rpc,
                &submit_stats,
                solution,
            )
            .await;
        }
    });

    // Spawn stats display task
    let display_stats = Arc::clone(&stats);
    let display_state = Arc::clone(&state);
    let stats_interval = Duration::from_secs(args.stats_interval);
    
    let stats_handle = tokio::spawn(async move {
        stats_display_loop(display_stats, display_state, stats_interval).await
    });

    // Wait for Ctrl+C
    info!("Miner started. Press Ctrl+C to stop.");
    tokio::signal::ctrl_c().await?;

    // Shutdown
    info!("Shutting down miner...");
    state.shutdown.store(true, Ordering::SeqCst);

    // Wait for tasks to complete
    poll_handle.abort();
    submit_handle.abort();
    stats_handle.abort();

    // Wait for mining thread
    let _ = mining_handle.join();

    // Print final stats
    let final_stats = stats.snapshot();
    info!("╔═══════════════════════════════════════════╗");
    info!("║            Final Statistics               ║");
    info!("╠═══════════════════════════════════════════╣");
    info!("║  Uptime:       {:<26} ║", stats.uptime_string());
    info!("║  Total hashes: {:<26} ║", final_stats.total_hashes);
    info!("║  Avg hashrate: {:<26} ║", MiningStats::format_hash_rate(final_stats.average_hash_rate));
    info!("║  Blocks found: {:<26} ║", final_stats.blocks_found);
    info!("╚═══════════════════════════════════════════╝");

    info!("Miner stopped.");
    Ok(())
}

/// Poll the node for new work
async fn work_polling_loop(
    state: Arc<MinerState>,
    rpc: Arc<RpcClient>,
    coinbase: Address,
    interval: Duration,
) {
    let mut consecutive_errors = 0u32;

    while !state.shutdown.load(Ordering::Relaxed) {
        match rpc.get_work(&coinbase).await {
            Ok(work) => {
                consecutive_errors = 0;
                let new_height = work.header.height;

                // Check if work has changed
                let should_update = {
                    let current = state.current_work.read();
                    match &*current {
                        Some(existing) => existing.header.height != new_height
                            || existing.header.prev_hash != work.header.prev_hash,
                        None => true,
                    }
                };

                if should_update {
                    info!(
                        "New work: height={}, difficulty={}, txs={}",
                        new_height, work.header.difficulty, work.tx_count
                    );

                    *state.current_work.write() = Some(work);
                    state.work_updated.store(true, Ordering::SeqCst);
                }
            }
            Err(e) => {
                consecutive_errors += 1;
                let backoff = Duration::from_millis(
                    (interval.as_millis() as u64) * consecutive_errors.min(10) as u64,
                );

                if consecutive_errors <= 3 {
                    debug!("Failed to get work: {} (retry in {:?})", e, backoff);
                } else {
                    warn!(
                        "Failed to get work (attempt {}): {} (retry in {:?})",
                        consecutive_errors, e, backoff
                    );
                }

                tokio::time::sleep(backoff).await;
                continue;
            }
        }

        tokio::time::sleep(interval).await;
    }
}

/// Main mining loop running on dedicated threads
fn mining_loop(
    state: Arc<MinerState>,
    stats: Arc<MiningStats>,
    num_threads: usize,
    pow_config: PoWConfig,
    solution_tx: mpsc::Sender<SolutionFound>,
) {
    let miner = Miner::new(pow_config).with_threads(num_threads);

    while !state.shutdown.load(Ordering::Relaxed) {
        // Wait for work
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

        // Mine in chunks to allow checking for work updates
        let chunk_size: u64 = 100_000;
        let mut start_nonce: u64 = 0;

        loop {
            // Check for shutdown or new work
            if state.shutdown.load(Ordering::Relaxed) {
                return;
            }

            if state.work_updated.load(Ordering::Relaxed) {
                debug!("Work updated, restarting mining");
                break;
            }

            let end_nonce = start_nonce.saturating_add(chunk_size);

            match miner.mine_range(&work.header, start_nonce, end_nonce) {
                Ok(MiningResult::Found { nonce, hash }) => {
                    info!("🎉 Found valid nonce: {} for height {}", nonce, work.header.height);

                    // Send solution
                    let solution = SolutionFound {
                        work_id: work.work_id.clone(),
                        nonce,
                        pow_hash: hash,
                    };

                    if let Err(e) = solution_tx.blocking_send(solution) {
                        error!("Failed to send solution: {}", e);
                    }

                    // Wait for new work
                    break;
                }
                Ok(MiningResult::Exhausted) => {
                    // Continue to next range
                    start_nonce = end_nonce;

                    // Wrap around if we've exhausted the nonce space
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

            // Update stats
            let miner_stats = miner.stats();
            stats.add_hashes(miner_stats.hashes);
        }
    }
}

/// Handle a found solution
async fn handle_solution(
    rpc: &RpcClient,
    stats: &MiningStats,
    solution: SolutionFound,
) {
    let work_solution = WorkSolution {
        work_id: solution.work_id,
        nonce: solution.nonce,
        pow_hash: format!("0x{}", hex::encode(solution.pow_hash)),
    };

    match rpc.submit_work(&work_solution).await {
        Ok(true) => {
            info!("✓ Block accepted by node!");
            stats.record_block_found();
        }
        Ok(false) => {
            warn!("✗ Block rejected by node (stale or invalid)");
            stats.record_block_rejected();
        }
        Err(e) => {
            error!("Failed to submit solution: {}", e);
            stats.record_block_rejected();
        }
    }
}

/// Display mining statistics periodically
async fn stats_display_loop(
    stats: Arc<MiningStats>,
    state: Arc<MinerState>,
    interval: Duration,
) {
    while !state.shutdown.load(Ordering::Relaxed) {
        tokio::time::sleep(interval).await;

        let snapshot = stats.snapshot();
        info!("{}", snapshot);
    }
}

/// Get the number of CPU cores
mod num_cpus {
    pub fn get() -> usize {
        std::thread::available_parallelism()
            .map(|p| p.get())
            .unwrap_or(1)
    }
}
