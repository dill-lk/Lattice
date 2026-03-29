//! Lattice Node - Full node implementation
//!
//! This binary wires together all Lattice components into a complete blockchain node.

mod config;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use clap::Parser;
use parking_lot::RwLock;
use tokio::sync::broadcast;
use tracing_subscriber::EnvFilter;

use config::{NodeConfig, parse_network};
use lattice_core::{Block, BlockHeader, Transaction, Address, Network, BlockHeight, Hash};
use lattice_consensus::{MinerBuilder, MiningResult, DifficultyAdjuster, PoWConfig};
use lattice_crypto::sha3_256;
use lattice_storage::{BlockStore, StateStore, MempoolStore};
use lattice_rpc::{RpcServer, RpcConfig as RpcServerConfig, RpcHandlers, ChainState};
use lattice_vm::Runtime;

// ============================================================================
// CLI Arguments
// ============================================================================

#[derive(Parser)]
#[command(name = "lattice-node")]
#[command(version, about = "Lattice blockchain full node", long_about = None)]
struct Args {
    /// Path to configuration file
    #[arg(short, long)]
    config: Option<PathBuf>,

    /// Data directory for blockchain storage
    #[arg(long)]
    datadir: Option<String>,

    /// Network to connect to (mainnet, testnet, devnet)
    #[arg(long)]
    network: Option<String>,

    /// RPC server port
    #[arg(long)]
    rpc_port: Option<u16>,

    /// RPC server host
    #[arg(long)]
    rpc_host: Option<String>,

    /// Disable RPC server
    #[arg(long)]
    no_rpc: bool,

    /// P2P listening port
    #[arg(long)]
    p2p_port: Option<u16>,

    /// Enable mining
    #[arg(long)]
    mine: bool,

    /// Mining threads (if mining enabled)
    #[arg(long)]
    mining_threads: Option<usize>,

    /// Coinbase address for mining rewards
    #[arg(long)]
    coinbase: Option<String>,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long)]
    log_level: Option<String>,

    /// Bootstrap nodes (comma-separated multiaddrs)
    #[arg(long)]
    bootnodes: Option<String>,

    /// Generate default config and exit
    #[arg(long)]
    init: bool,
}

// ============================================================================
// Node Events
// ============================================================================

/// Events that can occur within the node
#[derive(Debug, Clone)]
pub enum NodeEvent {
    /// New block received from network or mined
    NewBlock(Arc<Block>),
    /// New transaction received
    NewTransaction(Arc<Transaction>),
    /// Block mined successfully
    BlockMined(Arc<Block>),
    /// Peer connected
    PeerConnected(String),
    /// Peer disconnected
    PeerDisconnected(String),
    /// Sync status changed
    SyncStatusChanged(SyncStatus),
    /// Shutdown requested
    Shutdown,
}

/// Current sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Not syncing, fully synced
    Synced,
    /// Currently syncing
    Syncing { current: BlockHeight, target: BlockHeight },
    /// Initial sync from genesis
    InitialSync,
}

// ============================================================================
// Node State
// ============================================================================

/// Shared node state accessible by all components
pub struct NodeState {
    /// Network type
    pub network: Network,
    /// Block storage
    pub block_store: Arc<BlockStore>,
    /// State storage (accounts)
    pub state_store: Arc<StateStore>,
    /// Mempool storage
    pub mempool: Arc<MempoolStore>,
    /// Current chain tip height
    pub chain_height: RwLock<BlockHeight>,
    /// Current chain tip hash
    pub chain_tip: RwLock<Hash>,
    /// Sync status
    pub sync_status: RwLock<SyncStatus>,
    /// Difficulty adjuster
    pub difficulty: Arc<DifficultyAdjuster>,
    /// VM runtime for contract execution
    pub vm_runtime: Arc<Runtime>,
    /// Event broadcaster
    pub event_tx: broadcast::Sender<NodeEvent>,
}

impl NodeState {
    /// Get the current chain height
    pub fn height(&self) -> BlockHeight {
        *self.chain_height.read()
    }

    /// Get the current chain tip hash
    pub fn tip(&self) -> Hash {
        *self.chain_tip.read()
    }

    /// Update chain tip
    pub fn update_tip(&self, height: BlockHeight, hash: Hash) {
        *self.chain_height.write() = height;
        *self.chain_tip.write() = hash;
    }

    /// Broadcast a node event
    pub fn emit_event(&self, event: NodeEvent) {
        // Ignore errors if no receivers
        let _ = self.event_tx.send(event);
    }
}

// ============================================================================
// Genesis Block
// ============================================================================

/// Create genesis block for a network
fn create_genesis_block(network: Network) -> Block {
    let timestamp = match network {
        Network::Mainnet => 1700000000000, // Mainnet launch timestamp
        Network::Testnet => 1699000000000,
        Network::Devnet => 0,
    };

    let header = BlockHeader {
        version: 1,
        height: 0,
        prev_hash: [0u8; 32],
        tx_root: [0u8; 32],
        state_root: [0u8; 32],
        timestamp,
        difficulty: match network {
            Network::Mainnet => 1_000_000,
            Network::Testnet => 100_000,
            Network::Devnet => 1,
        },
        nonce: 0,
        coinbase: Address::zero(),
    };

    Block {
        header,
        transactions: Vec::new(),
    }
}

/// Load genesis block or create it if not present
fn load_or_create_genesis(
    block_store: &BlockStore,
    network: Network,
) -> anyhow::Result<Block> {
    // Check if genesis exists
    if let Ok(Some(block)) = block_store.get_by_height(0) {
        tracing::info!("Loaded existing genesis block");
        return Ok(block);
    }

    // Create genesis
    tracing::info!("Creating genesis block for {:?}", network);
    let genesis = create_genesis_block(network);
    
    block_store.put(&genesis)
        .context("Failed to store genesis block")?;
    
    tracing::info!("Genesis block created: {:?}", sha3_256(&borsh::to_vec(&genesis.header)?));
    Ok(genesis)
}

// ============================================================================
// Miner Task
// ============================================================================

/// Mining coordinator that produces new blocks
async fn run_miner(
    state: Arc<NodeState>,
    config: config::MiningConfig,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    if !config.enabled {
        tracing::debug!("Miner not enabled, task exiting");
        return;
    }

    let coinbase = match &config.coinbase {
        Some(addr) => {
            // Accept Base58Check format (output of `lattice-cli wallet address`)
            // or hex format (0x-prefixed or plain 40-char hex).
            if let Ok(a) = Address::from_base58(addr) {
                a
            } else {
                match hex::decode(addr.trim_start_matches("0x")) {
                    Ok(bytes) if bytes.len() == 20 => {
                        let mut arr = [0u8; 20];
                        arr.copy_from_slice(&bytes);
                        Address::from_bytes(arr)
                    }
                    _ => {
                        tracing::error!(
                            "Invalid coinbase address: {}. \
                             Use the address shown by `lattice-cli wallet address`.",
                            addr
                        );
                        return;
                    }
                }
            }
        }
        None => {
            tracing::error!("Mining enabled but no coinbase address provided");
            return;
        }
    };

    tracing::info!("Miner started with {} threads, coinbase: {}",
        config.threads, coinbase);

    let miner = MinerBuilder::new()
        .threads(config.threads)
        .build();

    loop {
        // Check for shutdown
        if shutdown_rx.try_recv().is_ok() {
            tracing::info!("Miner received shutdown signal");
            break;
        }

        // Don't mine if we're syncing
        if *state.sync_status.read() != SyncStatus::Synced {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        // Build block template
        let template = match build_block_template(&state, coinbase.clone()).await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("Failed to build block template: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        // Mine the block
        tracing::debug!("Mining block at height {}", template.header.height);
        
        let result = tokio::task::spawn_blocking({
            let miner = miner.clone();
            let header = template.header.clone();
            move || miner.mine(&header)
        }).await;

        match result {
            Ok(Ok(MiningResult::Found { nonce, hash })) => {
                let mut block = template;
                block.header.nonce = nonce;

                tracing::info!(
                    "⛏ Mined block {} with hash {:?}",
                    block.header.height,
                    hash
                );

                // Store and broadcast
                if let Err(e) = state.block_store.put(&block) {
                    tracing::error!("Failed to store mined block: {}", e);
                } else {
                    state.update_tip(block.header.height, hash);
                    state.emit_event(NodeEvent::BlockMined(Arc::new(block)));
                }
            }
            Ok(Ok(MiningResult::Cancelled)) => {
                tracing::debug!("Mining cancelled (new block arrived?)");
            }
            Ok(Ok(MiningResult::Exhausted)) => {
                tracing::warn!("Mining exhausted nonce space");
            }
            Ok(Err(e)) => {
                tracing::error!("Mining error: {}", e);
            }
            Err(e) => {
                tracing::error!("Mining task panicked: {}", e);
            }
        }

        // Small delay before next attempt
        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Build a block template for mining
async fn build_block_template(
    state: &NodeState,
    coinbase: Address,
) -> anyhow::Result<Block> {
    let parent_height = state.height();
    let parent_hash = state.tip();

    // Get transactions from mempool
    let txs = state.mempool.get_sorted_by_fee(1000)?;

    // Compute tx_root
    let tx_hashes: Vec<&[u8]> = txs.iter()
        .map(|tx| tx.data.as_slice())
        .collect();
    let tx_root = if tx_hashes.is_empty() {
        [0u8; 32]
    } else {
        sha3_256(&tx_hashes.concat())
    };

    // Get current difficulty from latest block, or use network default
    let difficulty = state.block_store.get_latest()
        .ok()
        .flatten()
        .map(|b| b.header.difficulty)
        .unwrap_or(match state.network {
            Network::Mainnet => 1_000_000,
            Network::Testnet => 100_000,
            Network::Devnet => 1,
        });

    let header = BlockHeader {
        version: 1,
        height: parent_height + 1,
        prev_hash: parent_hash,
        tx_root,
        state_root: [0u8; 32], // Will be computed after execution
        timestamp: std::time::SystemTime::now()
            .duration_since(std::time::UNIX_EPOCH)?
            .as_millis() as u64,
        difficulty,
        nonce: 0,
        coinbase,
    };

    Ok(Block {
        header,
        transactions: txs,
    })
}

// ============================================================================
// Event Loop
// ============================================================================

/// Main event loop that coordinates all node components
async fn run_event_loop(
    state: Arc<NodeState>,
    mut event_rx: broadcast::Receiver<NodeEvent>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    tracing::info!("Event loop started");

    loop {
        tokio::select! {
            // Handle shutdown
            _ = shutdown_rx.recv() => {
                tracing::info!("Event loop received shutdown signal");
                break;
            }
            
            // Handle node events
            event = event_rx.recv() => {
                match event {
                    Ok(NodeEvent::NewBlock(block)) => {
                        handle_new_block(&state, &block).await;
                    }
                    Ok(NodeEvent::NewTransaction(tx)) => {
                        handle_new_transaction(&state, &tx).await;
                    }
                    Ok(NodeEvent::BlockMined(block)) => {
                        tracing::info!("Block {} mined and stored", block.header.height);
                        // Remove mined transactions from mempool
                        for tx in &block.transactions {
                            let tx_hash = sha3_256(&borsh::to_vec(tx).unwrap_or_default());
                            let _ = state.mempool.remove(&tx_hash);
                        }
                    }
                    Ok(NodeEvent::PeerConnected(peer)) => {
                        tracing::info!("Peer connected: {}", peer);
                    }
                    Ok(NodeEvent::PeerDisconnected(peer)) => {
                        tracing::info!("Peer disconnected: {}", peer);
                    }
                    Ok(NodeEvent::SyncStatusChanged(status)) => {
                        *state.sync_status.write() = status;
                        match status {
                            SyncStatus::Synced => tracing::info!("Node synced"),
                            SyncStatus::Syncing { current, target } => {
                                tracing::info!("Syncing: {}/{}", current, target);
                            }
                            SyncStatus::InitialSync => tracing::info!("Starting initial sync"),
                        }
                    }
                    Ok(NodeEvent::Shutdown) => {
                        tracing::info!("Shutdown event received");
                        break;
                    }
                    Err(broadcast::error::RecvError::Lagged(n)) => {
                        tracing::warn!("Event loop lagged by {} events", n);
                    }
                    Err(broadcast::error::RecvError::Closed) => {
                        tracing::info!("Event channel closed");
                        break;
                    }
                }
            }
        }
    }

    tracing::info!("Event loop stopped");
}

/// Handle a new block received from network
async fn handle_new_block(state: &NodeState, block: &Block) {
    let block_hash = sha3_256(&borsh::to_vec(&block.header).unwrap_or_default());
    
    tracing::debug!(
        "Processing block {} hash={:?}",
        block.header.height,
        block_hash
    );

    // Validate block
    if let Err(e) = validate_block(state, block) {
        tracing::warn!("Invalid block: {}", e);
        return;
    }

    // Store block
    if let Err(e) = state.block_store.put(block) {
        tracing::error!("Failed to store block: {}", e);
        return;
    }

    // Update chain tip if this extends our chain
    if block.header.height > state.height() {
        state.update_tip(block.header.height, block_hash);
        tracing::info!("Chain tip updated to height {}", block.header.height);
        
        // Execute transactions and update state
        for tx in &block.transactions {
            if let Err(e) = execute_transaction(state, tx) {
                tracing::warn!("Transaction execution failed: {}", e);
            }
        }
    }
}

/// Validate a block before accepting it
fn validate_block(state: &NodeState, block: &Block) -> anyhow::Result<()> {
    // Check height
    let expected_height = state.height() + 1;
    if block.header.height != expected_height && block.header.height != 0 {
        // Allow orphan blocks for reorg handling
        if block.header.height <= state.height() {
            anyhow::bail!("Block height {} <= current height {}", 
                block.header.height, state.height());
        }
    }

    // Check previous hash
    if block.header.height > 0 && block.header.prev_hash != state.tip() {
        // Could be a fork, don't immediately reject
        tracing::debug!("Block parent doesn't match current tip");
    }

    // Verify PoW
    if !lattice_consensus::verify_pow(&block.header, &PoWConfig::default())
        .unwrap_or(false)
    {
        anyhow::bail!("Invalid proof of work");
    }

    Ok(())
}

/// Handle a new transaction from network or RPC
async fn handle_new_transaction(state: &NodeState, tx: &Transaction) {
    let tx_hash = sha3_256(&borsh::to_vec(tx).unwrap_or_default());
    
    tracing::debug!("Processing transaction {:?}", tx_hash);

    // Basic validation
    if let Err(e) = validate_transaction(tx) {
        tracing::warn!("Invalid transaction: {}", e);
        return;
    }

    // Add to mempool
    if let Err(e) = state.mempool.add(tx) {
        tracing::warn!("Failed to add transaction to mempool: {}", e);
    }
}

/// Validate a transaction
fn validate_transaction(tx: &Transaction) -> anyhow::Result<()> {
    // Check signature
    if tx.signature.is_empty() {
        anyhow::bail!("Missing signature");
    }

    // Verify signature using public key
    if !tx.public_key.is_empty() && !tx.verify_signature() {
        anyhow::bail!("Invalid signature");
    }

    Ok(())
}

/// Execute a transaction and update state
fn execute_transaction(_state: &NodeState, tx: &Transaction) -> anyhow::Result<()> {
    match tx.kind {
        lattice_core::TransactionKind::Transfer => {
            // Simple transfer: update balances
            // In a real implementation, this would use state_store
            tracing::debug!(
                "Transfer: {:?} -> {:?}, amount: {}",
                tx.from, tx.to, tx.amount
            );
        }
        lattice_core::TransactionKind::Deploy => {
            // Deploy contract
            tracing::debug!("Contract deployment from {:?}", tx.from);
            // Would use state.vm_runtime.deploy()
        }
        lattice_core::TransactionKind::Call => {
            // Call contract
            tracing::debug!("Contract call to {:?}", tx.to);
            // Would use state.vm_runtime.call()
        }
    }
    Ok(())
}

// ============================================================================
// RPC Server Integration
// ============================================================================

/// Start the RPC server
async fn start_rpc_server(
    config: config::RpcConfig,
    state: Arc<NodeState>,
) -> anyhow::Result<()> {
    if !config.enabled {
        tracing::info!("RPC server disabled");
        return Ok(());
    }

    let rpc_config = RpcServerConfig {
        host: config.host.clone(),
        port: config.port,
        cors_enabled: config.cors_enabled,
    };

    // Create chain state from node state
    let chain_state = create_chain_state(&state)?;
    
    let handlers = RpcHandlers::with_state(Arc::new(RwLock::new(chain_state)));
    let server = RpcServer::with_handlers(rpc_config, handlers);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    tracing::info!("Starting RPC server on {}", addr);

    // Run server in background
    tokio::spawn(async move {
        if let Err(e) = server.run().await {
            tracing::error!("RPC server error: {}", e);
        }
    });

    Ok(())
}

/// Create ChainState from NodeState for RPC handlers
fn create_chain_state(state: &NodeState) -> anyhow::Result<ChainState> {
    let mut chain_state = ChainState::new();

    // Load recent blocks
    let height = state.height();
    for h in height.saturating_sub(100)..=height {
        if let Ok(Some(block)) = state.block_store.get_by_height(h) {
            let hash = sha3_256(&borsh::to_vec(&block.header)?);
            // Index transactions
            for tx in &block.transactions {
                let tx_hash = sha3_256(&borsh::to_vec(tx).unwrap_or_default());
                chain_state.transactions.insert(tx_hash, (tx.clone(), Some(hash)));
            }
            if block.header.height > chain_state.height {
                chain_state.height = block.header.height;
            }
            chain_state.blocks_by_height.insert(block.header.height, block.clone());
            chain_state.blocks_by_hash.insert(hash, block);
        }
    }

    Ok(chain_state)
}

// ============================================================================
// P2P Networking (Placeholder)
// ============================================================================

/// Start P2P networking
async fn start_networking(
    config: config::P2pConfig,
    state: Arc<NodeState>,
    mut shutdown_rx: broadcast::Receiver<()>,
) -> anyhow::Result<()> {
    tracing::info!("Starting P2P networking on {}", config.listen_addr);
    
    // In a full implementation, this would:
    // 1. Create libp2p swarm with NetworkBehavior
    // 2. Connect to bootnodes
    // 3. Start gossipsub for block/tx propagation
    // 4. Handle sync requests
    
    // Placeholder: just log and wait for shutdown
    tokio::spawn(async move {
        tracing::info!("P2P network placeholder running");
        
        // Simulate peer discovery
        tokio::time::sleep(Duration::from_secs(2)).await;
        state.emit_event(NodeEvent::SyncStatusChanged(SyncStatus::Synced));
        
        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    tracing::info!("P2P network shutting down");
                    break;
                }
                _ = tokio::time::sleep(Duration::from_secs(60)) => {
                    tracing::debug!("P2P heartbeat");
                }
            }
        }
    });

    Ok(())
}

// ============================================================================
// Graceful Shutdown
// ============================================================================

/// Perform graceful shutdown of all components
async fn graceful_shutdown(
    state: Arc<NodeState>,
    shutdown_tx: broadcast::Sender<()>,
) {
    tracing::info!("Initiating graceful shutdown...");
    
    // Signal all components to stop
    let _ = shutdown_tx.send(());
    
    // Emit shutdown event
    state.emit_event(NodeEvent::Shutdown);
    
    // Give components time to cleanup
    tokio::time::sleep(Duration::from_millis(500)).await;
    
    // Flush storage
    tracing::info!("Flushing storage...");
    // block_store, state_store, mempool will flush on drop
    
    tracing::info!("Shutdown complete");
}

// ============================================================================
// Main Entry Point
// ============================================================================

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let args = Args::parse();

    // Handle --init flag
    if args.init {
        let config = NodeConfig::default();
        config.ensure_data_dir()?;
        config.save(&config.config_file_path())?;
        println!("Created default config at: {:?}", config.config_file_path());
        return Ok(());
    }

    // Load configuration
    let mut config = if let Some(config_path) = &args.config {
        NodeConfig::from_file(config_path)
            .with_context(|| format!("Failed to load config from {:?}", config_path))?
    } else {
        NodeConfig::default()
    };

    // Override config with CLI args
    if let Some(datadir) = &args.datadir {
        config.data_dir = PathBuf::from(expand_tilde(datadir));
    }
    if let Some(network) = &args.network {
        config.network = parse_network(network)?;
    }
    if let Some(port) = args.rpc_port {
        config.rpc.port = port;
    }
    if let Some(host) = &args.rpc_host {
        config.rpc.host = host.clone();
    }
    if args.no_rpc {
        config.rpc.enabled = false;
    }
    if let Some(port) = args.p2p_port {
        config.p2p.listen_addr.set_port(port);
    }
    if args.mine {
        config.mining.enabled = true;
    }
    if let Some(threads) = args.mining_threads {
        config.mining.threads = threads;
    }
    if let Some(coinbase) = &args.coinbase {
        config.mining.coinbase = Some(coinbase.clone());
    }
    if let Some(level) = &args.log_level {
        config.log_level = level.clone();
    }
    if let Some(bootnodes) = &args.bootnodes {
        config.p2p.bootnodes = bootnodes.split(',').map(|s| s.to_string()).collect();
    }

    // Initialize logging
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new(&config.log_level));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .init();

    // Print startup banner
    print_banner(&config);

    // Create data directories
    config.ensure_data_dir()
        .context("Failed to create data directories")?;

    // Initialize storage
    tracing::info!("Opening storage...");
    let block_store = Arc::new(
        BlockStore::open(config.blocks_db_path())
            .context("Failed to open block store")?
    );
    let state_store = Arc::new(
        StateStore::open(config.state_db_path())
            .context("Failed to open state store")?
    );
    let mempool = Arc::new(
        MempoolStore::open(config.mempool_db_path())
            .context("Failed to open mempool")?
    );

    // Load or create genesis
    let genesis = load_or_create_genesis(&block_store, config.network)?;
    let genesis_hash = sha3_256(&borsh::to_vec(&genesis.header)?);

    // Get current chain tip
    let (chain_height, chain_tip) = match block_store.get_latest()? {
        Some(block) => {
            let hash = sha3_256(&borsh::to_vec(&block.header)?);
            (block.header.height, hash)
        }
        None => (0, genesis_hash),
    };
    tracing::info!("Chain tip: height={}, hash={:?}", chain_height, chain_tip);

    // Create event channel
    let (event_tx, _) = broadcast::channel::<NodeEvent>(1024);

    // Create shutdown channel
    let (shutdown_tx, _) = broadcast::channel::<()>(1);

    // Create node state
    let state = Arc::new(NodeState {
        network: config.network,
        block_store: block_store.clone(),
        state_store: state_store.clone(),
        mempool: mempool.clone(),
        chain_height: RwLock::new(chain_height),
        chain_tip: RwLock::new(chain_tip),
        sync_status: RwLock::new(SyncStatus::InitialSync),
        difficulty: Arc::new(DifficultyAdjuster::new()),
        vm_runtime: Arc::new(Runtime::new()),
        event_tx: event_tx.clone(),
    });

    // Start event loop
    let event_loop = tokio::spawn(run_event_loop(
        state.clone(),
        event_tx.subscribe(),
        shutdown_tx.subscribe(),
    ));

    // Start P2P networking
    start_networking(
        config.p2p.clone(),
        state.clone(),
        shutdown_tx.subscribe(),
    ).await?;

    // Start RPC server
    start_rpc_server(config.rpc.clone(), state.clone()).await?;

    // Start miner
    let miner_handle = tokio::spawn(run_miner(
        state.clone(),
        config.mining.clone(),
        shutdown_tx.subscribe(),
    ));

    tracing::info!("Node started successfully");
    tracing::info!("Press Ctrl+C to shutdown");

    // Wait for shutdown signal
    tokio::signal::ctrl_c().await?;
    tracing::info!("Received shutdown signal");

    // Graceful shutdown
    graceful_shutdown(state.clone(), shutdown_tx).await;

    // Wait for tasks to complete
    let _ = tokio::time::timeout(Duration::from_secs(5), event_loop).await;
    let _ = tokio::time::timeout(Duration::from_secs(2), miner_handle).await;

    tracing::info!("Lattice node stopped");
    Ok(())
}

/// Expand ~ in paths
fn expand_tilde(path: &str) -> String {
    if path.starts_with("~/") || path == "~" {
        if let Some(home) = dirs::home_dir() {
            return path.replacen("~", &home.to_string_lossy(), 1);
        }
    }
    path.to_string()
}

/// Print startup banner
fn print_banner(config: &NodeConfig) {
    println!();
    println!("╔═══════════════════════════════════════════════════════════╗");
    println!("║                      LATTICE NODE                         ║");
    println!("║          Quantum-Resistant Blockchain Full Node           ║");
    println!("╚═══════════════════════════════════════════════════════════╝");
    println!();
    tracing::info!("Configuration:");
    tracing::info!("  Network:        {:?}", config.network);
    tracing::info!("  Data dir:       {:?}", config.data_dir);
    tracing::info!("  P2P:            {}", config.p2p.listen_addr);
    if config.rpc.enabled {
        tracing::info!("  RPC:            {}:{}", config.rpc.host, config.rpc.port);
    } else {
        tracing::info!("  RPC:            disabled");
    }
    if config.mining.enabled {
        tracing::info!("  Mining:         enabled ({} threads)", config.mining.threads);
        if let Some(coinbase) = &config.mining.coinbase {
            tracing::info!("  Coinbase:       {}", coinbase);
        }
    } else {
        tracing::info!("  Mining:         disabled");
    }
    println!();
}
