//! Genesis Block and Initial State Configuration
//!
//! This module handles the creation of the genesis block and initial blockchain state,
//! including the founder's genesis allocation (Developer Genesis Fee).

use crate::tokenomics::{
    self, VestingSchedule, FOUNDER_IMMEDIATE_AMOUNT,
    FOUNDER_VESTING_AMOUNT, FOUNDER_WALLET_ADDRESS, TOTAL_GENESIS_ALLOCATION,
    TOTAL_SUPPLY, TOKEN_SYMBOL,
};
use crate::{Account, Address, Amount, Block, BlockHeader, Network, State};
use sha3::{Digest, Sha3_256};
use serde::{Deserialize, Serialize};

/// Genesis configuration for a network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisConfig {
    /// Network identifier
    pub network: Network,
    /// Genesis timestamp (ms since Unix epoch)
    pub timestamp: u64,
    /// Initial difficulty
    pub difficulty: u64,
    /// Extra data (e.g., launch message)
    pub extra_data: String,
    /// Genesis allocations (pre-funded accounts)
    pub allocations: Vec<GenesisAccountAllocation>,
    /// Vesting schedules for locked tokens
    pub vesting_schedules: Vec<GenesisVestingSchedule>,
}

/// Pre-funded account in genesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAccountAllocation {
    /// Account address (Base58)
    pub address: String,
    /// Initial balance in LAT (will be converted to Latt)
    pub balance_lat: u64,
    /// Description
    pub description: String,
}

/// Vesting schedule defined at genesis
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisVestingSchedule {
    /// Beneficiary address (Base58)
    pub beneficiary: String,
    /// Total vesting amount in LAT
    pub amount_lat: u64,
    /// Vesting duration in months
    pub duration_months: u64,
    /// Cliff period in months
    pub cliff_months: u64,
    /// Description
    pub description: String,
}

impl GenesisConfig {
    /// Create the default mainnet genesis configuration
    pub fn mainnet() -> Self {
        Self {
            network: Network::Mainnet,
            // Genesis timestamp: 2025-01-01 00:00:00 UTC
            timestamp: 1735689600000,
            difficulty: 10,
            extra_data: "Lattice Mainnet Genesis - Quantum Resistant Blockchain".to_string(),
            allocations: vec![
                GenesisAccountAllocation {
                    address: FOUNDER_WALLET_ADDRESS.to_string(),
                    balance_lat: 500_000, // 500,000 LAT immediately available
                    description: "Founder immediate allocation".to_string(),
                },
            ],
            vesting_schedules: vec![
                GenesisVestingSchedule {
                    beneficiary: FOUNDER_WALLET_ADDRESS.to_string(),
                    amount_lat: 2_000_000, // 2,000,000 LAT vesting
                    duration_months: 24,    // 2 years
                    cliff_months: 0,        // No cliff
                    description: "Founder vesting allocation".to_string(),
                },
            ],
        }
    }

    /// Create testnet genesis configuration  
    pub fn testnet() -> Self {
        Self {
            network: Network::Testnet,
            timestamp: 1699000000000,
            difficulty: 100_000,
            extra_data: "Lattice Testnet Genesis".to_string(),
            allocations: vec![
                GenesisAccountAllocation {
                    address: FOUNDER_WALLET_ADDRESS.to_string(),
                    balance_lat: 1_000_000, // More for testing
                    description: "Testnet founder allocation".to_string(),
                },
            ],
            vesting_schedules: vec![],
        }
    }

    /// Create devnet genesis configuration (for local development)
    pub fn devnet() -> Self {
        Self {
            network: Network::Devnet,
            timestamp: 0,
            difficulty: 1,
            extra_data: "Lattice Devnet".to_string(),
            allocations: vec![
                GenesisAccountAllocation {
                    address: FOUNDER_WALLET_ADDRESS.to_string(),
                    balance_lat: 10_000_000, // Lots for dev testing
                    description: "Devnet test allocation".to_string(),
                },
            ],
            vesting_schedules: vec![],
        }
    }

    /// Get configuration for a specific network
    pub fn for_network(network: Network) -> Self {
        match network {
            Network::Mainnet => Self::mainnet(),
            Network::Testnet => Self::testnet(),
            Network::Devnet => Self::devnet(),
        }
    }
}

/// Result of genesis initialization
pub struct GenesisResult {
    /// The genesis block
    pub block: Block,
    /// Initial state with funded accounts
    pub state: State,
    /// Vesting schedules to track
    pub vesting_schedules: Vec<VestingSchedule>,
}

/// Errors that can occur during genesis creation
#[derive(Debug, thiserror::Error)]
pub enum GenesisError {
    #[error("invalid address: {0}")]
    InvalidAddress(String),
    #[error("allocation exceeds total supply")]
    AllocationExceedsSupply,
    #[error("duplicate allocation for address: {0}")]
    DuplicateAllocation(String),
}

/// Create genesis block and initial state from configuration
pub fn create_genesis(config: &GenesisConfig) -> Result<GenesisResult, GenesisError> {
    let mut state = State::new();
    let mut total_allocated: Amount = 0;

    // Process immediate allocations
    for allocation in &config.allocations {
        let address = Address::from_base58(&allocation.address)
            .map_err(|_| GenesisError::InvalidAddress(allocation.address.clone()))?;
        
        let amount = tokenomics::lat_to_latt(allocation.balance_lat);
        total_allocated += amount;
        
        if total_allocated > TOTAL_SUPPLY {
            return Err(GenesisError::AllocationExceedsSupply);
        }
        
        // Set initial balance
        state.set_account(address, Account::with_balance(amount));
    }

    // Process vesting schedules (tokens locked, not in circulation yet)
    let mut vesting_schedules = Vec::new();
    for schedule in &config.vesting_schedules {
        let beneficiary = Address::from_base58(&schedule.beneficiary)
            .map_err(|_| GenesisError::InvalidAddress(schedule.beneficiary.clone()))?;
        
        let amount = tokenomics::lat_to_latt(schedule.amount_lat);
        total_allocated += amount;
        
        if total_allocated > TOTAL_SUPPLY {
            return Err(GenesisError::AllocationExceedsSupply);
        }
        
        vesting_schedules.push(VestingSchedule {
            beneficiary,
            total_amount: amount,
            released_amount: 0,
            start_block: 0, // Starts from genesis
            duration_blocks: schedule.duration_months * tokenomics::BLOCKS_PER_MONTH,
            cliff_blocks: schedule.cliff_months * tokenomics::BLOCKS_PER_MONTH,
        });
    }

    // Calculate state root
    let state_root = state.root();

    // Create genesis block header
    let header = BlockHeader {
        version: 1,
        height: 0,
        prev_hash: [0u8; 32],
        tx_root: [0u8; 32], // No transactions in genesis
        state_root,
        timestamp: config.timestamp,
        difficulty: config.difficulty,
        nonce: calculate_genesis_nonce(&config.extra_data),
        coinbase: Address::zero(),
    };

    let block = Block {
        header,
        transactions: vec![],
    };

    Ok(GenesisResult {
        block,
        state,
        vesting_schedules,
    })
}

/// Calculate a deterministic nonce from extra data
fn calculate_genesis_nonce(extra_data: &str) -> u64 {
    let hash = Sha3_256::digest(extra_data.as_bytes());
    u64::from_le_bytes(hash[0..8].try_into().unwrap())
}

/// Genesis information for display/documentation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisInfo {
    pub network: String,
    pub block_hash: String,
    pub timestamp: u64,
    pub total_supply: String,
    pub genesis_allocation: String,
    pub founder_immediate: String,
    pub founder_vesting: String,
    pub vesting_duration: String,
    pub founder_address: String,
}

impl GenesisInfo {
    /// Get genesis information for a network
    pub fn for_network(network: Network) -> Result<Self, GenesisError> {
        let config = GenesisConfig::for_network(network);
        let result = create_genesis(&config)?;
        
        Ok(Self {
            network: format!("{:?}", network),
            block_hash: hex::encode(result.block.hash()),
            timestamp: config.timestamp,
            total_supply: format!("{} {}", tokenomics::latt_to_lat(TOTAL_SUPPLY), TOKEN_SYMBOL),
            genesis_allocation: format!(
                "{} {} ({}%)",
                tokenomics::latt_to_lat(TOTAL_GENESIS_ALLOCATION),
                TOKEN_SYMBOL,
                (TOTAL_GENESIS_ALLOCATION * 100) / TOTAL_SUPPLY
            ),
            founder_immediate: format!(
                "{} {}",
                tokenomics::latt_to_lat(FOUNDER_IMMEDIATE_AMOUNT),
                TOKEN_SYMBOL
            ),
            founder_vesting: format!(
                "{} {}",
                tokenomics::latt_to_lat(FOUNDER_VESTING_AMOUNT),
                TOKEN_SYMBOL
            ),
            vesting_duration: "24 months (2 years)".to_string(),
            founder_address: FOUNDER_WALLET_ADDRESS.to_string(),
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_mainnet_genesis() {
        let config = GenesisConfig::mainnet();
        let result = create_genesis(&config).expect("Genesis creation should succeed");
        
        // Check block
        assert_eq!(result.block.height(), 0);
        assert_eq!(result.block.header.prev_hash, [0u8; 32]);
        
        // Check founder allocation
        let founder = Address::from_base58(FOUNDER_WALLET_ADDRESS).unwrap();
        let balance = result.state.balance(&founder);
        assert_eq!(balance, FOUNDER_IMMEDIATE_AMOUNT);
        
        // Check vesting schedule
        assert_eq!(result.vesting_schedules.len(), 1);
        assert_eq!(result.vesting_schedules[0].total_amount, FOUNDER_VESTING_AMOUNT);
    }

    #[test]
    fn test_testnet_genesis() {
        let config = GenesisConfig::testnet();
        let result = create_genesis(&config).expect("Genesis creation should succeed");
        
        assert_eq!(result.block.height(), 0);
        assert!(result.vesting_schedules.is_empty()); // No vesting on testnet
    }

    #[test]
    fn test_devnet_genesis() {
        let config = GenesisConfig::devnet();
        let result = create_genesis(&config).expect("Genesis creation should succeed");
        
        // Devnet has easy difficulty
        assert_eq!(result.block.header.difficulty, 1);
    }

    #[test]
    fn test_total_allocation_within_supply() {
        let config = GenesisConfig::mainnet();
        let result = create_genesis(&config).unwrap();
        
        // Total immediate allocation
        let founder = Address::from_base58(FOUNDER_WALLET_ADDRESS).unwrap();
        let immediate = result.state.balance(&founder);
        
        // Total vesting
        let vesting_total: Amount = result.vesting_schedules
            .iter()
            .map(|v| v.total_amount)
            .sum();
        
        // Must not exceed total supply
        assert!(immediate + vesting_total <= TOTAL_SUPPLY);
        
        // Should equal TOTAL_GENESIS_ALLOCATION
        assert_eq!(immediate + vesting_total, TOTAL_GENESIS_ALLOCATION);
    }

    #[test]
    fn test_genesis_block_hash_deterministic() {
        let config = GenesisConfig::mainnet();
        let result1 = create_genesis(&config).unwrap();
        let result2 = create_genesis(&config).unwrap();
        
        assert_eq!(result1.block.hash(), result2.block.hash());
    }

    #[test]
    fn test_invalid_address_rejected() {
        let mut config = GenesisConfig::devnet();
        config.allocations.push(GenesisAccountAllocation {
            address: "invalid_address".to_string(),
            balance_lat: 1000,
            description: "Bad allocation".to_string(),
        });
        
        let result = create_genesis(&config);
        assert!(matches!(result, Err(GenesisError::InvalidAddress(_))));
    }

    #[test]
    fn test_over_allocation_rejected() {
        let mut config = GenesisConfig::devnet();
        config.allocations.push(GenesisAccountAllocation {
            address: FOUNDER_WALLET_ADDRESS.to_string(),
            balance_lat: 100_000_000, // Way more than total supply
            description: "Over allocation".to_string(),
        });
        
        let result = create_genesis(&config);
        assert!(matches!(result, Err(GenesisError::AllocationExceedsSupply)));
    }
}
