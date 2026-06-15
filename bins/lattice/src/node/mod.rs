//! Lattice Node - Full node implementation module
//!
//! This module wires together all Lattice components into a complete blockchain node.

pub mod config;

use std::net::SocketAddr;
use std::path::PathBuf;
use std::sync::Arc;
use std::time::Duration;

use anyhow::Context;
use libp2p::futures::StreamExt;
use libp2p::{identity, noise, swarm::SwarmEvent, tcp, yamux, Multiaddr, PeerId, SwarmBuilder};
use parking_lot::{Mutex, RwLock};
use tokio::sync::{broadcast, mpsc};
use tracing_subscriber::EnvFilter;

use crate::node::config::{parse_network, NodeConfig};
use lattice_consensus::{DifficultyAdjuster, MinerBuilder, MiningResult, PoWConfig};
use lattice_core::genesis::GenesisConfig;
use lattice_core::tokenomics::{lat_to_latt, BLOCK_REWARD};
use lattice_core::validation::{
    execute_block as execute_block_to_state, validate_block as validate_block_with_context,
    validate_transaction as validate_transaction_with_context, BlockValidationContext,
    TxValidationContext,
};
use lattice_core::{
    Account, Address, Block, BlockHeader, BlockHeight, Hash, Network, State, Transaction,
};
use lattice_crypto::sha3_256;
use lattice_network::{
    ChainSync, NetworkBehavior, NetworkBehaviorEvent, NetworkEvent as P2pEvent, PeerConfig,
    PeerManager, SyncConfig as NetSyncConfig, SyncRequest, SyncResponse,
};
use lattice_rpc::{ChainState, PeerSnapshot, RpcConfig as RpcServerConfig, RpcHandlers, RpcServer};
use lattice_storage::{BlockStore, MempoolStore, StateStore};
use lattice_vm::Runtime;

const MAINNET_GENESIS_DIFFICULTY: u64 = 20_000;
const MAINNET_DYNAMIC_DIFFICULTY_START_HEIGHT: u64 = 10;
const MAINNET_DYNAMIC_ADJUSTMENT_INTERVAL: u64 = 10;

/// CLI Arguments for Node (starts full node)
#[derive(clap::Args, Debug, Clone)]
pub struct NodeArgs {
    /// Path to configuration file
    #[arg(short, long)]
    pub config: Option<PathBuf>,

    /// Data directory for blockchain storage
    #[arg(long)]
    pub datadir: Option<String>,

    /// Network to connect to (mainnet, testnet, devnet)
    #[arg(long)]
    pub network: Option<String>,

    /// RPC server port
    #[arg(long)]
    pub rpc_port: Option<u16>,

    /// RPC server host
    #[arg(long)]
    pub rpc_host: Option<String>,

    /// Disable RPC server
    #[arg(long)]
    pub no_rpc: bool,

    /// P2P listening port
    #[arg(long)]
    pub p2p_port: Option<u16>,

    /// Enable mining
    #[arg(long)]
    pub mine: bool,

    /// Mining threads (if mining enabled)
    #[arg(long)]
    pub mining_threads: Option<usize>,

    /// Coinbase address for mining rewards
    #[arg(long)]
    pub coinbase: Option<String>,

    /// Log level (error, warn, info, debug, trace)
    #[arg(long)]
    pub log_level: Option<String>,

    /// Bootstrap nodes (comma-separated multiaddrs)
    #[arg(long)]
    pub bootnodes: Option<String>,

    /// Generate default config and exit
    #[arg(long)]
    pub init: bool,

    /// Use light PoW config (for testing on low-end hardware)
    #[arg(long)]
    pub light: bool,
}

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

#[derive(Debug, Clone)]
pub enum NetworkCommand {
    BroadcastBlock(Block),
    BroadcastTransaction(Transaction),
}

/// Current sync status
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SyncStatus {
    /// Not syncing, fully synced
    Synced,
    /// Currently syncing
    Syncing {
        current: BlockHeight,
        target: BlockHeight,
    },
    /// Initial sync from genesis
    InitialSync,
}

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
    /// Lightweight peer snapshots for operator UX / RPC status
    pub peer_infos: RwLock<Vec<PeerSnapshot>>,
    /// Shared RPC-visible chain state
    pub rpc_state: Arc<RwLock<ChainState>>,
    /// Networking command channel for broadcast actions
    pub network_tx: mpsc::UnboundedSender<NetworkCommand>,
    /// Difficulty adjuster
    pub difficulty: Arc<DifficultyAdjuster>,
    /// VM runtime for contract execution
    pub vm_runtime: Arc<Mutex<Runtime>>,
    /// Event broadcaster
    pub event_tx: broadcast::Sender<NodeEvent>,
    /// PoW configuration for block verification
    pub pow_config: PoWConfig,
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

/// Create genesis block for a network
fn create_genesis_block(network: Network) -> Block {
    let mut config = GenesisConfig::for_network(network);
    if matches!(network, Network::Mainnet) {
        config.difficulty = MAINNET_GENESIS_DIFFICULTY;
    }

    match lattice_core::genesis::create_genesis(&config) {
        Ok(result) => result.block,
        Err(err) => {
            tracing::error!(error = %err, "Failed to build configured genesis, using fallback");
            Block {
                header: BlockHeader {
                    version: 1,
                    height: 0,
                    prev_hash: [0u8; 32],
                    tx_root: [0u8; 32],
                    state_root: [0u8; 32],
                    timestamp: config.timestamp,
                    difficulty: config.difficulty.max(1),
                    nonce: 0,
                    coinbase: Address::zero(),
                },
                transactions: Vec::new(),
            }
        }
    }
}

/// Load genesis block or create it if not present
fn load_or_create_genesis(
    block_store: &BlockStore,
    state_store: &StateStore,
    network: Network,
) -> anyhow::Result<Block> {
    if let Ok(Some(block)) = block_store.get_by_height(0) {
        tracing::info!("Loaded existing genesis block");
        return Ok(block);
    }

    tracing::info!("Creating genesis block for {:?}", network);
    let genesis = create_genesis_block(network);

    block_store
        .put(&genesis)
        .context("Failed to store genesis block")?;

    let mut config = GenesisConfig::for_network(network);
    if matches!(network, Network::Mainnet) {
        config.difficulty = MAINNET_GENESIS_DIFFICULTY;
    }
    for allocation in &config.allocations {
        let address = Address::from_base58(&allocation.address).map_err(|_| {
            anyhow::anyhow!("Invalid genesis allocation address: {}", allocation.address)
        })?;
        let balance = lat_to_latt(allocation.balance_lat);
        state_store
            .set_account(&address, &Account::with_balance(balance))
            .context("Failed to seed genesis allocation into state store")?;
    }

    tracing::info!(
        "Genesis block created: {:?}",
        sha3_256(&borsh::to_vec(&genesis.header)?)
    );
    Ok(genesis)
}

fn current_timestamp_ms() -> anyhow::Result<u64> {
    Ok(std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)?
        .as_millis() as u64)
}

fn load_current_state(state_store: &StateStore) -> anyhow::Result<State> {
    state_store
        .load_state()
        .map_err(|e| anyhow::anyhow!("Failed to load current state: {e}"))
}

fn persist_state(state_store: &StateStore, state: &State) -> anyhow::Result<()> {
    state_store
        .replace_state(state)
        .map_err(|e| anyhow::anyhow!("Failed to persist updated state: {e}"))
}

fn block_context_from_header(header: &BlockHeader) -> lattice_vm::BlockContext {
    lattice_vm::BlockContext {
        height: header.height,
        timestamp: header.timestamp,
        difficulty: header.difficulty,
        gas_limit: 10_000_000,
        coinbase: header.coinbase.clone(),
        prev_hash: header.prev_hash,
    }
}

fn seed_runtime_from_state_snapshot(
    runtime: &mut Runtime,
    state_store: &StateStore,
    chain_state: &State,
) -> anyhow::Result<()> {
    runtime.reset_chain_view();

    for (address, account) in chain_state.iter_accounts() {
        runtime.set_balance(address, account.balance);
        if account.is_contract() {
            if let Some(code) = state_store
                .get_code(&account.code_hash)
                .map_err(|e| anyhow::anyhow!("Failed to load contract code for {}: {}", address, e))?
            {
                runtime
                    .register_contract(address.clone(), code)
                    .map_err(|e| anyhow::anyhow!("Failed to register runtime contract {}: {}", address, e))?;
            }
            if let Some(storage) = state_store
                .get_contract_storage(address)
                .map_err(|e| anyhow::anyhow!("Failed to load contract storage for {}: {}", address, e))?
            {
                runtime.set_contract_storage(address, storage);
            }
        }
    }

    Ok(())
}

fn sync_runtime_from_node_state(state: &NodeState) -> anyhow::Result<()> {
    let chain_state = load_current_state(&state.state_store)?;
    let mut runtime = state.vm_runtime.lock();
    seed_runtime_from_state_snapshot(&mut runtime, &state.state_store, &chain_state)
}

fn apply_contract_effects(
    state_store: &StateStore,
    state_snapshot: &mut State,
    block: &Block,
    persist_runtime_state: bool,
) -> anyhow::Result<()> {
    let mut runtime = Runtime::new();
    seed_runtime_from_state_snapshot(&mut runtime, state_store, state_snapshot)?;
    let block_ctx = block_context_from_header(&block.header);

    for tx in &block.transactions {
        match tx.kind {
            lattice_core::TransactionKind::Deploy => {
                let deployment = runtime
                    .deploy(
                        tx.data.clone(),
                        tx.from.clone(),
                        tx.amount,
                        tx.gas_limit,
                        block_ctx.clone(),
                        Vec::new(),
                    )
                    .map_err(|e| anyhow::anyhow!("Contract deployment failed: {}", e))?;

                if persist_runtime_state {
                    state_store
                        .put_code(&deployment.code_hash, &tx.data)
                        .map_err(|e| anyhow::anyhow!("Failed to persist deployed code: {}", e))?;
                }

                let mut account = state_snapshot.get_account(&deployment.address);
                account.code_hash = deployment.code_hash;
                account.storage_root = runtime.contract_storage_root(&deployment.address);
                state_snapshot.set_account(deployment.address.clone(), account);

                let storage = runtime.contract_storage_snapshot(&deployment.address);
                if persist_runtime_state {
                    state_store
                        .put_contract_storage(&deployment.address, &storage)
                        .map_err(|e| anyhow::anyhow!("Failed to persist deployed storage: {}", e))?;
                }
            }
            lattice_core::TransactionKind::Call => {
                let result = runtime
                    .call(
                        tx.to.clone(),
                        tx.from.clone(),
                        tx.amount,
                        tx.data.clone(),
                        tx.gas_limit,
                        block_ctx.clone(),
                    )
                    .map_err(|e| anyhow::anyhow!("Contract call failed: {}", e))?;

                if !result.success {
                    anyhow::bail!(
                        "Contract call returned failure: {}",
                        result.error.unwrap_or_else(|| "unknown contract failure".to_string())
                    );
                }

                let mut account = state_snapshot.get_account(&tx.to);
                account.storage_root = runtime.contract_storage_root(&tx.to);
                state_snapshot.set_account(tx.to.clone(), account);

                let storage = runtime.contract_storage_snapshot(&tx.to);
                if persist_runtime_state {
                    state_store
                        .put_contract_storage(&tx.to, &storage)
                        .map_err(|e| anyhow::anyhow!("Failed to persist contract storage: {}", e))?;
                }
            }
            lattice_core::TransactionKind::Transfer => {}
        }
    }

    Ok(())
}

fn apply_block_to_chain(state: &NodeState, block: &Block) -> anyhow::Result<Hash> {
    let pre_state = load_current_state(&state.state_store)?;
    let parent_block = if block.header.height == 0 {
        None
    } else {
        state
            .block_store
            .get_by_height(block.header.height.saturating_sub(1))
            .map_err(|e| anyhow::anyhow!("Failed to load parent block: {e}"))?
    };

    let ctx = BlockValidationContext {
        parent_block: parent_block.as_ref(),
        current_timestamp: current_timestamp_ms()?,
        state: &pre_state,
    };

    validate_block_with_context(block, &ctx)
        .map_err(|e| anyhow::anyhow!("Core block validation failed: {e}"))?;

    if !lattice_consensus::verify_pow(&block.header, &state.pow_config)
        .map_err(|e| anyhow::anyhow!("PoW verification failed: {e}"))?
    {
        anyhow::bail!("Invalid proof of work");
    }

    let mut post_state = execute_block_to_state(block, pre_state)
        .map_err(|e| anyhow::anyhow!("Block execution failed: {e}"))?;
    apply_contract_effects(&state.state_store, &mut post_state, block, true)?;
    let computed_state_root = post_state.root();

    if block.header.state_root != computed_state_root {
        anyhow::bail!(
            "State root mismatch: header={:?} computed={:?}",
            block.header.state_root,
            computed_state_root
        );
    }

    persist_state(&state.state_store, &post_state)?;
    state
        .state_store
        .create_snapshot(block.header.height)
        .map_err(|e| anyhow::anyhow!("Failed to create state snapshot: {e}"))?;
    state
        .block_store
        .put(block)
        .map_err(|e| anyhow::anyhow!("Failed to store block: {e}"))?;

    Ok(block.hash())
}

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
                             Use the address shown by `wallet address`.",
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

    tracing::info!(
        "Miner started with {} threads, coinbase: {}",
        config.threads,
        coinbase
    );

    let miner = MinerBuilder::new().threads(config.threads).build();

    loop {
        if shutdown_rx.try_recv().is_ok() {
            tracing::info!("Miner received shutdown signal");
            break;
        }

        if *state.sync_status.read() != SyncStatus::Synced {
            tokio::time::sleep(Duration::from_secs(1)).await;
            continue;
        }

        let template = match build_block_template(&state, coinbase.clone()).await {
            Ok(t) => t,
            Err(e) => {
                tracing::warn!("Failed to build block template: {}", e);
                tokio::time::sleep(Duration::from_secs(1)).await;
                continue;
            }
        };

        tracing::debug!("Mining block at height {}", template.header.height);

        let result = tokio::task::spawn_blocking({
            let miner = miner.clone();
            let header = template.header.clone();
            move || miner.mine(&header)
        })
        .await;

        match result {
            Ok(Ok(MiningResult::Found { nonce, hash })) => {
                let mut block = template;
                block.header.nonce = nonce;

                tracing::info!("⛏ Mined block {} with hash {:?}", block.header.height, hash);

                match apply_block_to_chain(&state, &block) {
                    Ok(applied_hash) => {
                        state.update_tip(block.header.height, applied_hash);
                        state.emit_event(NodeEvent::BlockMined(Arc::new(block)));
                    }
                    Err(e) => {
                        tracing::error!("Failed to apply mined block: {}", e);
                    }
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

        tokio::time::sleep(Duration::from_millis(100)).await;
    }
}

/// Build a block template for mining.
async fn build_block_template(state: &NodeState, coinbase: Address) -> anyhow::Result<Block> {
    let parent_height = state.height();
    let parent_hash = state.tip();
    let mut working_state = load_current_state(&state.state_store)?;
    let candidate_txs = state.mempool.get_sorted_by_fee(1000)?;
    let timestamp = current_timestamp_ms()?;

    let mut selected_txs = Vec::new();
    for tx in candidate_txs {
        let tx_ctx = TxValidationContext {
            state: &working_state,
            chain_id: state.network.chain_id(),
            current_timestamp: timestamp,
        };

        if validate_transaction_with_context(&tx, &tx_ctx).is_err() {
            continue;
        }

        if lattice_core::validation::execute_transaction(&tx, &mut working_state).is_err() {
            continue;
        }

        selected_txs.push(tx);
    }

    let total_fees: u128 = selected_txs.iter().map(|tx| tx.fee).sum();
    let coinbase_account = working_state.get_account_mut(&coinbase);
    coinbase_account.balance = coinbase_account
        .balance
        .checked_add(BLOCK_REWARD)
        .and_then(|value| value.checked_add(total_fees))
        .ok_or_else(|| anyhow::anyhow!("Coinbase reward overflow while building block template"))?;

    let tx_root = Block::calculate_tx_root(&selected_txs);

    let difficulty = match state.network {
        Network::Mainnet => {
            let next_height = parent_height + 1;

            if next_height <= MAINNET_DYNAMIC_DIFFICULTY_START_HEIGHT {
                MAINNET_GENESIS_DIFFICULTY
            } else {
                let latest = state
                    .block_store
                    .get_latest()
                    .ok()
                    .flatten()
                    .map(|b| b.header.difficulty)
                    .unwrap_or(MAINNET_GENESIS_DIFFICULTY);

                if parent_height != 0
                    && parent_height.is_multiple_of(MAINNET_DYNAMIC_ADJUSTMENT_INTERVAL)
                {
                    let start_height = parent_height
                        .saturating_sub(MAINNET_DYNAMIC_ADJUSTMENT_INTERVAL)
                        .saturating_add(1);

                    let mut blocks =
                        Vec::with_capacity(MAINNET_DYNAMIC_ADJUSTMENT_INTERVAL as usize);
                    for h in start_height..=parent_height {
                        if let Ok(Some(block)) = state.block_store.get_by_height(h) {
                            blocks.push(block);
                        }
                    }

                    state
                        .difficulty
                        .adjust_from_blocks(&blocks)
                        .unwrap_or(latest)
                } else {
                    latest
                }
            }
        }
        Network::Testnet => state
            .block_store
            .get_latest()
            .ok()
            .flatten()
            .map(|b| b.header.difficulty)
            .unwrap_or(5),
        Network::Devnet => state
            .block_store
            .get_latest()
            .ok()
            .flatten()
            .map(|b| b.header.difficulty)
            .unwrap_or(1),
    };

    let mut header = BlockHeader {
        version: 1,
        height: parent_height + 1,
        prev_hash: parent_hash,
        tx_root,
        state_root: [0u8; 32],
        timestamp,
        difficulty,
        nonce: 0,
        coinbase,
    };

    let temp_block = Block {
        header: header.clone(),
        transactions: selected_txs.clone(),
    };
    apply_contract_effects(&state.state_store, &mut working_state, &temp_block, false)?;
    header.state_root = working_state.root();

    Ok(Block {
        header,
        transactions: selected_txs,
    })
}

/// Main event loop that coordinates all node components
async fn run_event_loop(
    state: Arc<NodeState>,
    mut event_rx: broadcast::Receiver<NodeEvent>,
    mut shutdown_rx: broadcast::Receiver<()>,
) {
    tracing::info!("Event loop started");

    loop {
        tokio::select! {
            _ = shutdown_rx.recv() => {
                tracing::info!("Event loop received shutdown signal");
                break;
            }

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
                        for tx in &block.transactions {
                            let _ = state.mempool.remove(&tx.hash());
                        }
                        let _ = state.network_tx.send(NetworkCommand::BroadcastBlock((*block).clone()));
                        let _ = refresh_rpc_state_from_node(&state);
                    }
                    Ok(NodeEvent::PeerConnected(peer)) => {
                        tracing::info!("Peer connected: {}", peer);
                        let _ = refresh_rpc_state_from_node(&state);
                    }
                    Ok(NodeEvent::PeerDisconnected(peer)) => {
                        tracing::info!("Peer disconnected: {}", peer);
                        let _ = refresh_rpc_state_from_node(&state);
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
                        let _ = refresh_rpc_state_from_node(&state);
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

/// Handle a new block received from the network.
async fn handle_new_block(state: &NodeState, block: &Block) {
    tracing::debug!(
        "Processing block {} hash={:?}",
        block.header.height,
        block.hash()
    );

    match apply_block_to_chain(state, block) {
        Ok(block_hash) => {
            if block.header.height > state.height() {
                state.update_tip(block.header.height, block_hash);
            }

            for tx in &block.transactions {
                let _ = state.mempool.remove(&tx.hash());
            }

            let _ = refresh_rpc_state_from_node(state);
            tracing::info!("Chain tip updated to height {}", block.header.height);
        }
        Err(e) => {
            tracing::warn!("Rejected block {}: {}", block.header.height, e);
        }
    }
}

/// Handle a new transaction from network or RPC.
async fn handle_new_transaction(state: &NodeState, tx: &Transaction) {
    tracing::debug!("Processing transaction {:?}", tx.hash());

    let current_state = match load_current_state(&state.state_store) {
        Ok(state_snapshot) => state_snapshot,
        Err(e) => {
            tracing::warn!("Failed to load state for transaction validation: {}", e);
            return;
        }
    };

    let ctx = TxValidationContext {
        state: &current_state,
        chain_id: state.network.chain_id(),
        current_timestamp: current_timestamp_ms().unwrap_or_default(),
    };

    if let Err(e) = validate_transaction_with_context(tx, &ctx) {
        tracing::warn!("Invalid transaction: {}", e);
        return;
    }

    if let Err(e) = state.mempool.add(tx) {
        tracing::warn!("Failed to add transaction to mempool: {}", e);
    } else {
        let _ = state
            .network_tx
            .send(NetworkCommand::BroadcastTransaction(tx.clone()));
        let _ = refresh_rpc_state_from_node(state);
    }
}

/// Start the RPC server
async fn start_rpc_server(config: config::RpcConfig, state: Arc<NodeState>) -> anyhow::Result<()> {
    if !config.enabled {
        tracing::info!("RPC server disabled");
        return Ok(());
    }

    let rpc_config = RpcServerConfig {
        host: config.host.clone(),
        port: config.port,
        cors_enabled: config.cors_enabled,
    };

    refresh_rpc_state_from_node(&state)?;

    let handlers = RpcHandlers::with_state(state.rpc_state.clone());
    let server = RpcServer::with_handlers(rpc_config, handlers);

    let addr: SocketAddr = format!("{}:{}", config.host, config.port).parse()?;
    tracing::info!("Starting RPC server on {}", addr);

    tokio::spawn(async move {
        if let Err(e) = server.run().await {
            tracing::error!("RPC server error: {}", e);
        }
    });

    Ok(())
}

/// Refresh the live RPC-visible chain state from the node state.
fn refresh_rpc_state_from_node(state: &NodeState) -> anyhow::Result<()> {
    sync_runtime_from_node_state(state)?;
    let mut chain_state = ChainState::with_pow_config(state.pow_config.clone());
    chain_state.runtime = Some(state.vm_runtime.clone());

    let height = state.height();
    for h in 0..=height {
        if let Ok(Some(block)) = state.block_store.get_by_height(h) {
            let hash = sha3_256(&borsh::to_vec(&block.header)?);
            for tx in &block.transactions {
                let tx_hash = tx.hash();
                chain_state
                    .transactions
                    .insert(tx_hash, (tx.clone(), Some(hash)));
            }
            if block.header.height > chain_state.height {
                chain_state.height = block.header.height;
            }
            chain_state
                .blocks_by_height
                .insert(block.header.height, block.clone());
            chain_state.blocks_by_hash.insert(hash, block);
        }
    }

    if let Ok(addresses) = state.state_store.list_accounts() {
        for address in addresses {
            if let Ok(Some(account)) = state.state_store.get_account(&address) {
                chain_state.balances.insert(address, account.balance);
            }
        }
    }

    if let Ok(pending) = state.mempool.get_sorted_by_fee(256) {
        chain_state.pending_txs = pending;
    }

    let sync_status = *state.sync_status.read();
    match sync_status {
        SyncStatus::Synced => {
            chain_state.syncing = false;
            chain_state.sync_current = state.height();
            chain_state.sync_target = state.height();
        }
        SyncStatus::Syncing { current, target } => {
            chain_state.syncing = true;
            chain_state.sync_current = current;
            chain_state.sync_target = target;
        }
        SyncStatus::InitialSync => {
            chain_state.syncing = true;
            chain_state.sync_current = state.height();
            chain_state.sync_target = state.height().max(1);
        }
    }

    chain_state.peer_infos = state.peer_infos.read().clone();
    *state.rpc_state.write() = chain_state;
    Ok(())
}

fn socketaddr_to_multiaddr(addr: SocketAddr) -> anyhow::Result<Multiaddr> {
    match addr.ip() {
        std::net::IpAddr::V4(ip) => format!("/ip4/{}/tcp/{}", ip, addr.port())
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid listen multiaddr: {e}")),
        std::net::IpAddr::V6(ip) => format!("/ip6/{}/tcp/{}", ip, addr.port())
            .parse()
            .map_err(|e| anyhow::anyhow!("Invalid listen multiaddr: {e}")),
    }
}

fn sync_peer_snapshots_from_manager(peer_manager: &PeerManager, state: &NodeState) {
    let mut snapshots = Vec::new();
    for peer_id in peer_manager.connected_peers() {
        if let Some(info) = peer_manager.get_peer(&peer_id) {
            let address = info
                .addresses
                .first()
                .map(|addr| addr.to_string())
                .unwrap_or_else(|| peer_id.to_string());
            snapshots.push(PeerSnapshot {
                id: peer_id.to_string(),
                address,
                latency_ms: 0,
                score: info.score.score,
            });
        }
    }
    *state.peer_infos.write() = snapshots;
}

fn build_sync_status_response(state: &NodeState) -> SyncResponse {
    SyncResponse::Status {
        protocol_version: lattice_network::PROTOCOL_VERSION.to_string(),
        best_height: state.height(),
        best_hash: state.tip(),
        genesis_hash: state
            .block_store
            .get_genesis_hash()
            .ok()
            .flatten()
            .unwrap_or([0u8; 32]),
    }
}

fn build_headers_response(state: &NodeState, start_hash: Hash, max_headers: u32) -> SyncResponse {
    let headers = match state.block_store.get(&start_hash) {
        Ok(Some(start_block)) => state
            .block_store
            .get_range(
                start_block.height().saturating_add(1),
                start_block
                    .height()
                    .saturating_add(max_headers as u64),
            )
            .unwrap_or_default()
            .into_iter()
            .map(|block| block.header)
            .collect(),
        _ => Vec::new(),
    };
    SyncResponse::Headers(headers)
}

fn build_blocks_response(state: &NodeState, hashes: Vec<Hash>) -> SyncResponse {
    let mut blocks = Vec::new();
    for hash in hashes {
        if let Ok(Some(block)) = state.block_store.get(&hash) {
            blocks.push(block);
        }
    }
    SyncResponse::Blocks(blocks)
}

fn build_pooled_transactions_response(state: &NodeState, hashes: Vec<Hash>) -> SyncResponse {
    let mut txs = Vec::new();
    for hash in hashes {
        if let Ok(Some(tx)) = state.mempool.get(&hash) {
            txs.push(tx);
        }
    }
    SyncResponse::PooledTransactions(txs)
}

fn dispatch_sync_requests(
    behaviour: &mut NetworkBehavior,
    sync: &ChainSync,
    requests: Vec<(PeerId, SyncRequest)>,
) {
    for (peer, request) in requests {
        if let Ok(request_id) = behaviour.send_sync_request(&peer, request.clone()) {
            sync.register_request(request_id, peer, &request);
        }
    }
}

fn import_ready_blocks(state: &NodeState, sync: &ChainSync) {
    while let Some(block) = sync.next_block_to_import() {
        if let Ok(hash) = apply_block_to_chain(state, &block) {
            state.update_tip(block.header.height, hash);
            sync.update_local_state(block.header.height, hash);
            state.emit_event(NodeEvent::SyncStatusChanged(SyncStatus::Syncing {
                current: sync.local_height(),
                target: sync.target_height(),
            }));
        }
    }

    if !sync.is_syncing() || sync.progress() >= 1.0 {
        state.emit_event(NodeEvent::SyncStatusChanged(SyncStatus::Synced));
    }
}

fn handle_p2p_event(
    event: P2pEvent,
    state: &Arc<NodeState>,
    behaviour: &mut NetworkBehavior,
    peer_manager: &PeerManager,
    sync: &ChainSync,
    response_channel: Option<libp2p::request_response::ResponseChannel<Vec<u8>>>,
) {
    match event {
        P2pEvent::GossipBlock(block) => {
            state.emit_event(NodeEvent::NewBlock(Arc::new(block)));
        }
        P2pEvent::GossipTransaction(tx) => {
            state.emit_event(NodeEvent::NewTransaction(Arc::new(tx)));
        }
        P2pEvent::GossipBlockHeader(header) => {
            tracing::debug!(height = header.height, "Received block header gossip");
        }
        P2pEvent::PeerDiscovered(peer_id) => {
            peer_manager.add_peer(peer_id, None);
            sync_peer_snapshots_from_manager(peer_manager, state);
            let _ = refresh_rpc_state_from_node(state);
        }
        P2pEvent::PeerExpired(peer_id) => {
            peer_manager.on_mdns_expired(&peer_id);
            sync_peer_snapshots_from_manager(peer_manager, state);
            let _ = refresh_rpc_state_from_node(state);
        }
        P2pEvent::SyncRequest {
            peer,
            request,
            ..
        } => {
            if let Some(channel) = response_channel {
                let response = match request {
                    SyncRequest::GetStatus => build_sync_status_response(state),
                    SyncRequest::GetHeaders {
                        start_hash,
                        max_headers,
                    } => build_headers_response(state, start_hash, max_headers),
                    SyncRequest::GetBlocks { hashes } => build_blocks_response(state, hashes),
                    SyncRequest::GetPooledTransactions { hashes } => {
                        build_pooled_transactions_response(state, hashes)
                    }
                };
                let _ = behaviour.send_sync_response(channel, response);
                peer_manager.reward_peer(&peer, 1);
            }
        }
        P2pEvent::SyncResponse {
            peer,
            request_id,
            response,
        } => {
            sync.complete_request(request_id);
            let next_requests = match response {
                SyncResponse::Status {
                    best_height,
                    best_hash,
                    genesis_hash,
                    ..
                } => sync
                    .on_status_response(peer, best_height, best_hash, genesis_hash, peer_manager)
                    .into_iter()
                    .collect(),
                SyncResponse::Headers(headers) => sync.on_headers_response(peer, headers, peer_manager),
                SyncResponse::Blocks(blocks) => sync.on_blocks_response(peer, blocks, peer_manager),
                SyncResponse::PooledTransactions(txs) => {
                    for tx in txs {
                        state.emit_event(NodeEvent::NewTransaction(Arc::new(tx)));
                    }
                    Vec::new()
                }
                SyncResponse::Error(error) => {
                    tracing::warn!(%peer, %error, "Received sync error response");
                    Vec::new()
                }
            };
            dispatch_sync_requests(behaviour, sync, next_requests);
            import_ready_blocks(state, sync);
        }
        P2pEvent::SyncRequestFailed {
            request_id, ..
        } => {
            let requests = sync.on_request_failed(request_id, peer_manager);
            dispatch_sync_requests(behaviour, sync, requests);
        }
    }
}

/// Start P2P networking with a real libp2p swarm.
async fn start_networking(
    config: config::P2pConfig,
    state: Arc<NodeState>,
    mut shutdown_rx: broadcast::Receiver<()>,
    mut network_rx: mpsc::UnboundedReceiver<NetworkCommand>,
) -> anyhow::Result<()> {
    tracing::info!("Starting P2P networking on {}", config.listen_addr);

    let peer_manager = Arc::new(PeerManager::new(PeerConfig {
        max_peers: config.max_peers,
        enable_mdns: config.enable_mdns,
        ..PeerConfig::default()
    }));

    let local_key = identity::Keypair::generate_ed25519();
    let local_peer_id = local_key.public().to_peer_id();
    tracing::info!(%local_peer_id, "Initialized libp2p identity");

    let mut behaviour = NetworkBehavior::new(&local_key, config.enable_mdns)
        .map_err(|e| anyhow::anyhow!("Failed to create network behaviour: {e}"))?;
    behaviour
        .subscribe_topics()
        .map_err(|e| anyhow::anyhow!("Failed to subscribe gossip topics: {e}"))?;

    let mut swarm = SwarmBuilder::with_existing_identity(local_key)
        .with_tokio()
        .with_tcp(tcp::Config::default(), noise::Config::new, yamux::Config::default)
        .map_err(|e| anyhow::anyhow!("Failed to configure TCP transport: {e}"))?
        .with_behaviour(|_| behaviour)
        .map_err(|e| anyhow::anyhow!("Failed to attach network behaviour: {e}"))?
        .build();

    let listen_addr = socketaddr_to_multiaddr(config.listen_addr)?;
    swarm
        .listen_on(listen_addr.clone())
        .map_err(|e| anyhow::anyhow!("Failed to start listening: {e}"))?;

    for bootnode in &config.bootnodes {
        match bootnode.parse::<Multiaddr>() {
            Ok(addr) => {
                let _ = swarm.dial(addr.clone());
                tracing::info!(%addr, "Dialing configured bootnode");
            }
            Err(e) => tracing::warn!(%bootnode, %e, "Invalid bootnode multiaddr"),
        }
    }

    let genesis_hash = state
        .block_store
        .get_genesis_hash()?
        .unwrap_or([0u8; 32]);
    let sync = ChainSync::new(
        NetSyncConfig::default(),
        genesis_hash,
        state.height(),
        state.tip(),
    );

    sync_peer_snapshots_from_manager(&peer_manager, &state);
    let _ = refresh_rpc_state_from_node(&state);

    tokio::spawn(async move {
        let mut housekeeping = tokio::time::interval(Duration::from_secs(5));

        loop {
            tokio::select! {
                _ = shutdown_rx.recv() => {
                    tracing::info!("P2P network shutting down");
                    break;
                }
                Some(command) = network_rx.recv() => {
                    match command {
                        NetworkCommand::BroadcastBlock(block) => {
                            let _ = swarm.behaviour_mut().publish_block(block);
                        }
                        NetworkCommand::BroadcastTransaction(tx) => {
                            let _ = swarm.behaviour_mut().publish_transaction(tx);
                        }
                    }
                }
                _ = housekeeping.tick() => {
                    let timeout_requests = sync.check_timeouts(&peer_manager);
                    dispatch_sync_requests(swarm.behaviour_mut(), &sync, timeout_requests);
                    if sync.is_stalled() {
                        sync.reset_after_stall();
                    }
                    sync_peer_snapshots_from_manager(&peer_manager, &state);
                    let _ = refresh_rpc_state_from_node(&state);
                }
                event = swarm.select_next_some() => {
                    match event {
                        SwarmEvent::NewListenAddr { address, .. } => {
                            tracing::info!(%address, "Listening for peers");
                            sync_peer_snapshots_from_manager(&peer_manager, &state);
                            let _ = refresh_rpc_state_from_node(&state);
                            if config.bootnodes.is_empty() {
                                state.emit_event(NodeEvent::SyncStatusChanged(SyncStatus::Synced));
                            }
                        }
                        SwarmEvent::ConnectionEstablished { peer_id, endpoint, .. } => {
                            let inbound = endpoint.is_listener();
                            peer_manager.on_peer_connected(peer_id, inbound);
                            sync_peer_snapshots_from_manager(&peer_manager, &state);
                            let _ = refresh_rpc_state_from_node(&state);
                            state.emit_event(NodeEvent::PeerConnected(peer_id.to_string()));
                            let requests = sync.start_sync(&peer_manager);
                            dispatch_sync_requests(swarm.behaviour_mut(), &sync, requests);
                        }
                        SwarmEvent::ConnectionClosed { peer_id, endpoint, .. } => {
                            let inbound = endpoint.is_listener();
                            peer_manager.on_peer_disconnected(&peer_id, inbound);
                            sync_peer_snapshots_from_manager(&peer_manager, &state);
                            let _ = refresh_rpc_state_from_node(&state);
                            state.emit_event(NodeEvent::PeerDisconnected(peer_id.to_string()));
                        }
                        SwarmEvent::Behaviour(NetworkBehaviorEvent::Gossipsub(event)) => {
                            let parsed = {
                                let behaviour = swarm.behaviour();
                                behaviour.process_gossipsub_event(event)
                            };
                            if let Some(net_event) = parsed {
                                handle_p2p_event(net_event, &state, swarm.behaviour_mut(), &peer_manager, &sync, None);
                            }
                        }
                        SwarmEvent::Behaviour(NetworkBehaviorEvent::Mdns(event)) => {
                            let parsed = {
                                let behaviour = swarm.behaviour();
                                behaviour.process_mdns_event(event)
                            };
                            for net_event in parsed {
                                handle_p2p_event(net_event, &state, swarm.behaviour_mut(), &peer_manager, &sync, None);
                            }
                        }
                        SwarmEvent::Behaviour(NetworkBehaviorEvent::Sync(event)) => {
                            let parsed = {
                                let behaviour = swarm.behaviour();
                                behaviour.process_sync_event(event)
                            };
                            if let Some((net_event, response_channel)) = parsed {
                                handle_p2p_event(net_event, &state, swarm.behaviour_mut(), &peer_manager, &sync, response_channel);
                            }
                        }
                        _ => {}
                    }
                }
            }
        }
    });

    Ok(())
}

/// Perform graceful shutdown of all components
async fn graceful_shutdown(state: Arc<NodeState>, shutdown_tx: broadcast::Sender<()>) {
    tracing::info!("Initiating graceful shutdown...");

    let _ = shutdown_tx.send(());

    state.emit_event(NodeEvent::Shutdown);

    tokio::time::sleep(Duration::from_millis(500)).await;

    tracing::info!("Flushing storage...");
    tracing::info!("Shutdown complete");
}

/// Main entry point for the node runner
pub async fn run_node(args: NodeArgs) -> anyhow::Result<()> {
    if args.init {
        let config = NodeConfig::default();
        config.ensure_data_dir()?;
        config.save(&config.config_file_path())?;
        println!("Created default config at: {:?}", config.config_file_path());
        return Ok(());
    }

    let mut config = if let Some(config_path) = &args.config {
        NodeConfig::from_file(config_path)
            .with_context(|| format!("Failed to load config from {:?}", config_path))?
    } else {
        NodeConfig::default()
    };

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

    let env_filter =
        EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new(&config.log_level));
    tracing_subscriber::fmt()
        .with_env_filter(env_filter)
        .with_target(true)
        .with_thread_ids(true)
        .init();

    print_banner(&config);

    config
        .ensure_data_dir()
        .context("Failed to create data directories")?;

    tracing::info!("Opening storage...");
    let block_store =
        Arc::new(BlockStore::open(config.blocks_db_path()).context("Failed to open block store")?);
    let state_store =
        Arc::new(StateStore::open(config.state_db_path()).context("Failed to open state store")?);
    let mempool =
        Arc::new(MempoolStore::open(config.mempool_db_path()).context("Failed to open mempool")?);

    let genesis = load_or_create_genesis(&block_store, &state_store, config.network)?;
    let genesis_hash = sha3_256(&borsh::to_vec(&genesis.header)?);

    let (chain_height, chain_tip) = match block_store.get_latest()? {
        Some(block) => {
            let hash = sha3_256(&borsh::to_vec(&block.header)?);
            (block.header.height, hash)
        }
        None => (0, genesis_hash),
    };
    tracing::info!("Chain tip: height={}, hash={:?}", chain_height, chain_tip);

    let (event_tx, _) = broadcast::channel::<NodeEvent>(1024);
    let (shutdown_tx, _) = broadcast::channel::<()>(1);
    let (network_tx, network_rx) = mpsc::unbounded_channel::<NetworkCommand>();

    let pow_config = if args.light {
        tracing::warn!("Using light PoW config (testing only!)");
        PoWConfig::light()
    } else {
        match config.network {
            Network::Devnet => {
                tracing::info!("Using devnet PoW config (fast blocks)");
                PoWConfig::devnet()
            }
            Network::Testnet => {
                tracing::info!("Using testnet PoW config (moderate speed)");
                PoWConfig::testnet()
            }
            Network::Mainnet => {
                tracing::info!("Using mainnet PoW config (full security)");
                PoWConfig::mainnet()
            }
        }
    };

    let state = Arc::new(NodeState {
        network: config.network,
        block_store: block_store.clone(),
        state_store: state_store.clone(),
        mempool: mempool.clone(),
        chain_height: RwLock::new(chain_height),
        chain_tip: RwLock::new(chain_tip),
        sync_status: RwLock::new(SyncStatus::InitialSync),
        peer_infos: RwLock::new(Vec::new()),
        rpc_state: Arc::new(RwLock::new(ChainState::with_pow_config(pow_config.clone()))),
        network_tx,
        difficulty: Arc::new(match config.network {
            Network::Mainnet => DifficultyAdjuster::with_params(
                lattice_consensus::TARGET_BLOCK_TIME_MS,
                MAINNET_DYNAMIC_ADJUSTMENT_INTERVAL,
                lattice_consensus::MAX_ADJUSTMENT_FACTOR,
            ),
            _ => DifficultyAdjuster::new(),
        }),
        vm_runtime: Arc::new(Mutex::new(Runtime::new())),
        event_tx: event_tx.clone(),
        pow_config,
    });

    let _ = refresh_rpc_state_from_node(&state);

    let event_loop = tokio::spawn(run_event_loop(
        state.clone(),
        event_tx.subscribe(),
        shutdown_tx.subscribe(),
    ));

    start_networking(
        config.p2p.clone(),
        state.clone(),
        shutdown_tx.subscribe(),
        network_rx,
    )
    .await?;

    start_rpc_server(config.rpc.clone(), state.clone()).await?;

    let miner_handle = tokio::spawn(run_miner(
        state.clone(),
        config.mining.clone(),
        shutdown_tx.subscribe(),
    ));

    tracing::info!("Node started successfully");
    tracing::info!("Press Ctrl+C to shutdown");

    tokio::signal::ctrl_c().await?;
    tracing::info!("Received shutdown signal");

    graceful_shutdown(state.clone(), shutdown_tx).await;

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

/// Print startup banner.
fn print_banner(config: &NodeConfig) {
    use colored::Colorize;

    let title = if config.mining.enabled {
        "▲ lattice miner-node — local argon2 execution online"
    } else {
        "▲ lattice node — listening on sharded network mesh"
    };

    println!();
    println!("{}", title.bold().cyan());
    println!("{}", "─".repeat(62).dimmed());
    println!("  {:<12} {:?}", "network".dimmed(), config.network);
    println!("  {:<12} {:?}", "data".dimmed(), config.data_dir);
    println!("  {:<12} {}", "p2p".dimmed(), config.p2p.listen_addr);
    if config.rpc.enabled {
        println!("  {:<12} {}:{}", "rpc".dimmed(), config.rpc.host, config.rpc.port);
    }
    if config.mining.enabled {
        println!("  {:<12} {} threads", "workers".dimmed(), config.mining.threads);
        if let Some(coinbase) = &config.mining.coinbase {
            println!("  {:<12} {}", "target".dimmed(), coinbase);
        }
    }
    println!();
}
