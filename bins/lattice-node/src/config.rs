//! Node configuration module
//!
//! Handles loading configuration from files, CLI arguments, and defaults.

use std::net::{IpAddr, Ipv4Addr, SocketAddr};
use std::path::PathBuf;

use lattice_core::Network;
use serde::{Deserialize, Serialize};

/// Complete node configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct NodeConfig {
    /// Data directory for blockchain storage
    pub data_dir: PathBuf,

    /// Network to connect to (mainnet, testnet, devnet)
    pub network: Network,

    /// P2P networking configuration
    pub p2p: P2pConfig,

    /// RPC server configuration
    pub rpc: RpcConfig,

    /// Mining configuration
    pub mining: MiningConfig,

    /// Storage configuration
    pub storage: StorageConfig,

    /// Mempool configuration
    pub mempool: MempoolConfig,

    /// Sync configuration
    pub sync: SyncConfig,

    /// Logging level
    pub log_level: String,
}

impl Default for NodeConfig {
    fn default() -> Self {
        Self {
            data_dir: default_data_dir(),
            network: Network::Mainnet,
            p2p: P2pConfig::default(),
            rpc: RpcConfig::default(),
            mining: MiningConfig::default(),
            storage: StorageConfig::default(),
            mempool: MempoolConfig::default(),
            sync: SyncConfig::default(),
            log_level: "info".to_string(),
        }
    }
}

impl NodeConfig {
    /// Load configuration from a TOML file
    pub fn from_file(path: &PathBuf) -> anyhow::Result<Self> {
        let contents = std::fs::read_to_string(path)?;
        let config: NodeConfig = toml::from_str(&contents)?;
        Ok(config)
    }

    /// Save configuration to a TOML file
    pub fn save(&self, path: &PathBuf) -> anyhow::Result<()> {
        let contents = toml::to_string_pretty(self)?;
        std::fs::write(path, contents)?;
        Ok(())
    }

    /// Get the path to the blocks database
    pub fn blocks_db_path(&self) -> PathBuf {
        self.data_dir.join("blocks")
    }

    /// Get the path to the state database
    pub fn state_db_path(&self) -> PathBuf {
        self.data_dir.join("state")
    }

    /// Get the path to the mempool database
    pub fn mempool_db_path(&self) -> PathBuf {
        self.data_dir.join("mempool")
    }

    /// Get the path to the keystore
    pub fn keystore_path(&self) -> PathBuf {
        self.data_dir.join("keystore")
    }

    /// Get the path to the config file within data dir
    pub fn config_file_path(&self) -> PathBuf {
        self.data_dir.join("config.toml")
    }

    /// Create the data directory if it doesn't exist
    pub fn ensure_data_dir(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.data_dir)?;
        std::fs::create_dir_all(self.blocks_db_path())?;
        std::fs::create_dir_all(self.state_db_path())?;
        std::fs::create_dir_all(self.mempool_db_path())?;
        std::fs::create_dir_all(self.keystore_path())?;
        Ok(())
    }
}

/// P2P networking configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct P2pConfig {
    /// Listen address for P2P connections
    pub listen_addr: SocketAddr,

    /// Maximum number of peers
    pub max_peers: usize,

    /// Bootstrap nodes to connect to
    pub bootnodes: Vec<String>,

    /// Enable mDNS for local peer discovery
    pub enable_mdns: bool,

    /// Node identity keypair path (optional)
    pub identity_path: Option<PathBuf>,

    /// Connection timeout in seconds
    pub connection_timeout_secs: u64,

    /// Peer scoring enabled
    pub enable_peer_scoring: bool,
}

impl Default for P2pConfig {
    fn default() -> Self {
        Self {
            listen_addr: SocketAddr::new(IpAddr::V4(Ipv4Addr::new(0, 0, 0, 0)), 30303),
            max_peers: 50,
            bootnodes: Vec::new(),
            enable_mdns: true,
            identity_path: None,
            connection_timeout_secs: 30,
            enable_peer_scoring: true,
        }
    }
}

/// RPC server configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct RpcConfig {
    /// Enable RPC server
    pub enabled: bool,

    /// Host to bind RPC server
    pub host: String,

    /// Port for RPC server
    pub port: u16,

    /// Enable CORS
    pub cors_enabled: bool,

    /// Allowed CORS origins
    pub cors_origins: Vec<String>,

    /// Maximum request body size in bytes
    pub max_request_size: usize,

    /// Request timeout in seconds
    pub request_timeout_secs: u64,
}

impl Default for RpcConfig {
    fn default() -> Self {
        Self {
            enabled: true,
            host: "127.0.0.1".to_string(),
            port: 8545,
            cors_enabled: true,
            cors_origins: vec!["*".to_string()],
            max_request_size: 5 * 1024 * 1024, // 5 MB
            request_timeout_secs: 30,
        }
    }
}

/// Mining configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MiningConfig {
    /// Enable mining
    pub enabled: bool,

    /// Number of mining threads
    pub threads: usize,

    /// Coinbase address for rewards (hex-encoded)
    pub coinbase: Option<String>,

    /// Extra data to include in blocks
    pub extra_data: Vec<u8>,

    /// Target gas limit per block
    pub target_gas_limit: u64,

    /// Minimum gas price to accept transactions
    pub min_gas_price: u64,
}

impl Default for MiningConfig {
    fn default() -> Self {
        Self {
            enabled: false,
            threads: 1,
            coinbase: None,
            extra_data: Vec::new(),
            target_gas_limit: 30_000_000,
            min_gas_price: 1,
        }
    }
}

/// Storage configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct StorageConfig {
    /// RocksDB cache size in MB
    pub cache_size_mb: usize,

    /// Enable compression
    pub compression: bool,

    /// Maximum open files
    pub max_open_files: i32,

    /// Write buffer size in MB
    pub write_buffer_size_mb: usize,
}

impl Default for StorageConfig {
    fn default() -> Self {
        Self {
            cache_size_mb: 512,
            compression: true,
            max_open_files: 1000,
            write_buffer_size_mb: 64,
        }
    }
}

/// Mempool configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MempoolConfig {
    /// Maximum number of transactions
    pub max_size: usize,

    /// Maximum total size in bytes
    pub max_size_bytes: usize,

    /// Minimum gas price to accept
    pub min_gas_price: u64,

    /// Transaction expiration time in seconds
    pub tx_lifetime_secs: u64,
}

impl Default for MempoolConfig {
    fn default() -> Self {
        Self {
            max_size: 10_000,
            max_size_bytes: 256 * 1024 * 1024, // 256 MB
            min_gas_price: 1,
            tx_lifetime_secs: 3600, // 1 hour
        }
    }
}

/// Chain sync configuration
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SyncConfig {
    /// Enable fast sync (download state snapshots)
    pub fast_sync: bool,

    /// Maximum blocks to download in parallel
    pub parallel_blocks: usize,

    /// Sync batch size
    pub batch_size: usize,

    /// Sync timeout in seconds
    pub timeout_secs: u64,
}

impl Default for SyncConfig {
    fn default() -> Self {
        Self {
            fast_sync: false,
            parallel_blocks: 16,
            batch_size: 128,
            timeout_secs: 30,
        }
    }
}

/// Get the default data directory based on platform
fn default_data_dir() -> PathBuf {
    if cfg!(windows) {
        dirs::data_local_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Lattice")
    } else if cfg!(target_os = "macos") {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join("Library")
            .join("Lattice")
    } else {
        dirs::home_dir()
            .unwrap_or_else(|| PathBuf::from("."))
            .join(".lattice")
    }
}

/// Parse network from string
pub fn parse_network(s: &str) -> anyhow::Result<Network> {
    match s.to_lowercase().as_str() {
        "mainnet" | "main" => Ok(Network::Mainnet),
        "testnet" | "test" => Ok(Network::Testnet),
        "devnet" | "dev" => Ok(Network::Devnet),
        _ => anyhow::bail!("Unknown network: {}. Use mainnet, testnet, or devnet", s),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_config() {
        let config = NodeConfig::default();
        assert_eq!(config.rpc.port, 8545);
        assert_eq!(config.p2p.listen_addr.port(), 30303);
        assert!(!config.mining.enabled);
    }

    #[test]
    fn test_parse_network() {
        assert!(matches!(parse_network("mainnet").unwrap(), Network::Mainnet));
        assert!(matches!(parse_network("testnet").unwrap(), Network::Testnet));
        assert!(matches!(parse_network("devnet").unwrap(), Network::Devnet));
        assert!(parse_network("invalid").is_err());
    }
}
