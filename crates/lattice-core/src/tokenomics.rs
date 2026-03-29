//! Lattice Tokenomics - Supply, Genesis Allocation, and Vesting
//!
//! Defines the economic parameters of the Lattice blockchain:
//! - Total supply: 50,000,000 LAT
//! - Decimals: 8 (1 LAT = 100,000,000 Latt)
//! - Genesis allocation with vesting schedule
//! - Block reward configuration

use crate::{Address, Amount, BlockHeight};
use serde::{Deserialize, Serialize};

// ============================================================================
// Core Constants
// ============================================================================

/// Symbol for the native token
pub const TOKEN_SYMBOL: &str = "LAT";

/// Name of the smallest unit (like Satoshi for Bitcoin)
pub const SMALLEST_UNIT_NAME: &str = "Latt";

/// Number of decimal places (8, like Bitcoin)
pub const DECIMALS: u8 = 8;

/// Conversion factor: 1 LAT = 10^8 Latt
pub const LATT_PER_LAT: Amount = 100_000_000;

/// Total supply in base units (50,000,000 LAT)
/// 50_000_000 * 10^8 = 5_000_000_000_000_000 Latt
pub const TOTAL_SUPPLY: Amount = 50_000_000 * LATT_PER_LAT;

/// Block reward: 10 LAT per block (in base units)
pub const BLOCK_REWARD: Amount = 10 * LATT_PER_LAT;

/// Target block time in milliseconds (15 seconds)
pub const TARGET_BLOCK_TIME_MS: u64 = 15_000;

/// Blocks per day (approximate: 86400 / 15 = 5760)
pub const BLOCKS_PER_DAY: u64 = 5760;

/// Blocks per month (approximate: 30 days)
pub const BLOCKS_PER_MONTH: u64 = BLOCKS_PER_DAY * 30;

/// Blocks per year (approximate: 365 days)
pub const BLOCKS_PER_YEAR: u64 = BLOCKS_PER_DAY * 365;

// ============================================================================
// Genesis Allocation
// ============================================================================

/// Founder/Developer wallet address (Base58 encoded)
/// Address: 13jXqXbCSghDF2KgyFQdtw8SvbJvpEyhft
pub const FOUNDER_WALLET_ADDRESS: &str = "13jXqXbCSghDF2KgyFQdtw8SvbJvpEyhft";

/// Total developer genesis allocation: 2,500,000 LAT (5% of total supply)
pub const TOTAL_GENESIS_ALLOCATION: Amount = 2_500_000 * LATT_PER_LAT;

/// Immediately available to founder: 500,000 LAT (for initial expenses)
pub const FOUNDER_IMMEDIATE_AMOUNT: Amount = 500_000 * LATT_PER_LAT;

/// Vesting amount: 2,000,000 LAT (locked with vesting schedule)
pub const FOUNDER_VESTING_AMOUNT: Amount = 2_000_000 * LATT_PER_LAT;

/// Vesting duration in months (24 months = 2 years)
pub const VESTING_DURATION_MONTHS: u64 = 24;

/// Vesting release percentage per month (approximately 4.167%)
/// We use basis points for precision: 417 = 4.17%
pub const VESTING_MONTHLY_RELEASE_BPS: u64 = 417;

/// Vesting cliff period in months (0 = no cliff, tokens start vesting immediately)
pub const VESTING_CLIFF_MONTHS: u64 = 0;

// ============================================================================
// Utility Functions
// ============================================================================

/// Convert LAT to Latt (base units)
#[inline]
pub const fn lat_to_latt(lat: u64) -> Amount {
    (lat as Amount) * LATT_PER_LAT
}

/// Convert Latt to LAT (may lose precision for fractional amounts)
#[inline]
pub const fn latt_to_lat(latt: Amount) -> u64 {
    (latt / LATT_PER_LAT) as u64
}

/// Format amount in LAT with proper decimal places
pub fn format_lat(latt: Amount) -> String {
    let whole = latt / LATT_PER_LAT;
    let frac = latt % LATT_PER_LAT;
    
    if frac == 0 {
        format!("{} {}", whole, TOKEN_SYMBOL)
    } else {
        // Format with up to 8 decimal places, trim trailing zeros
        let frac_str = format!("{:08}", frac);
        let trimmed = frac_str.trim_end_matches('0');
        format!("{}.{} {}", whole, trimmed, TOKEN_SYMBOL)
    }
}

/// Parse LAT string to Latt amount
pub fn parse_lat(s: &str) -> Result<Amount, ParseAmountError> {
    let s = s.trim().to_uppercase();
    let s = s.strip_suffix("LAT").unwrap_or(&s).trim();
    
    if s.contains('.') {
        let parts: Vec<&str> = s.split('.').collect();
        if parts.len() != 2 {
            return Err(ParseAmountError::InvalidFormat);
        }
        
        let whole: Amount = parts[0].parse()
            .map_err(|_| ParseAmountError::InvalidNumber)?;
        
        let frac_str = parts[1];
        if frac_str.len() > 8 {
            return Err(ParseAmountError::TooManyDecimals);
        }
        
        // Pad to 8 decimal places
        let padded = format!("{:0<8}", frac_str);
        let frac: Amount = padded.parse()
            .map_err(|_| ParseAmountError::InvalidNumber)?;
        
        Ok(whole * LATT_PER_LAT + frac)
    } else {
        let whole: Amount = s.parse()
            .map_err(|_| ParseAmountError::InvalidNumber)?;
        Ok(whole * LATT_PER_LAT)
    }
}

/// Error parsing amount string
#[derive(Debug, Clone, thiserror::Error)]
pub enum ParseAmountError {
    #[error("invalid number format")]
    InvalidFormat,
    #[error("invalid number")]
    InvalidNumber,
    #[error("too many decimal places (max 8)")]
    TooManyDecimals,
}

// ============================================================================
// Vesting Schedule
// ============================================================================

/// Vesting schedule information
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct VestingSchedule {
    /// Beneficiary address
    pub beneficiary: Address,
    /// Total vesting amount
    pub total_amount: Amount,
    /// Amount already released
    pub released_amount: Amount,
    /// Start block height
    pub start_block: BlockHeight,
    /// Duration in blocks
    pub duration_blocks: u64,
    /// Cliff period in blocks (tokens locked until this block)
    pub cliff_blocks: u64,
}

impl VestingSchedule {
    /// Create the founder's vesting schedule
    pub fn founder_schedule(start_block: BlockHeight) -> Result<Self, VestingError> {
        let beneficiary = Address::from_base58(FOUNDER_WALLET_ADDRESS)
            .map_err(|_| VestingError::InvalidAddress)?;
        
        Ok(Self {
            beneficiary,
            total_amount: FOUNDER_VESTING_AMOUNT,
            released_amount: 0,
            start_block,
            duration_blocks: VESTING_DURATION_MONTHS * BLOCKS_PER_MONTH,
            cliff_blocks: VESTING_CLIFF_MONTHS * BLOCKS_PER_MONTH,
        })
    }
    
    /// Calculate amount vested (unlocked) at a given block height
    pub fn vested_amount(&self, current_block: BlockHeight) -> Amount {
        if current_block < self.start_block {
            return 0;
        }
        
        let elapsed = current_block - self.start_block;
        
        // Check cliff
        if elapsed < self.cliff_blocks {
            return 0;
        }
        
        // Calculate vested amount (linear vesting)
        if elapsed >= self.duration_blocks {
            self.total_amount
        } else {
            // Linear vesting: (elapsed / duration) * total
            // Use u128 to avoid overflow
            let vested = (self.total_amount as u128 * elapsed as u128) 
                / self.duration_blocks as u128;
            vested as Amount
        }
    }
    
    /// Calculate amount available to claim (vested but not yet released)
    pub fn claimable_amount(&self, current_block: BlockHeight) -> Amount {
        let vested = self.vested_amount(current_block);
        vested.saturating_sub(self.released_amount)
    }
    
    /// Record a release of tokens
    pub fn release(&mut self, amount: Amount) -> Result<(), VestingError> {
        if amount > self.claimable_amount(0) {
            return Err(VestingError::InsufficientVestedAmount);
        }
        self.released_amount += amount;
        Ok(())
    }
    
    /// Check if vesting is complete
    pub fn is_complete(&self) -> bool {
        self.released_amount >= self.total_amount
    }
}

/// Vesting error types
#[derive(Debug, Clone, thiserror::Error)]
pub enum VestingError {
    #[error("invalid beneficiary address")]
    InvalidAddress,
    #[error("insufficient vested amount")]
    InsufficientVestedAmount,
    #[error("vesting schedule not found")]
    ScheduleNotFound,
}

// ============================================================================
// Genesis State
// ============================================================================

/// Genesis allocation entry
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct GenesisAllocation {
    /// Recipient address
    pub address: Address,
    /// Allocated amount in base units (Latt)
    pub amount: Amount,
    /// Description of allocation
    pub description: String,
}

/// Get all genesis allocations for initial state
pub fn get_genesis_allocations() -> Result<Vec<GenesisAllocation>, GenesisError> {
    let founder_address = Address::from_base58(FOUNDER_WALLET_ADDRESS)
        .map_err(|_| GenesisError::InvalidFounderAddress)?;
    
    Ok(vec![
        GenesisAllocation {
            address: founder_address,
            amount: FOUNDER_IMMEDIATE_AMOUNT,
            description: "Founder immediate allocation (500,000 LAT)".to_string(),
        },
        // Note: Vesting tokens (2,000,000 LAT) are tracked separately via VestingSchedule
        // and released over time, not allocated in genesis state directly
    ])
}

/// Genesis configuration error
#[derive(Debug, Clone, thiserror::Error)]
pub enum GenesisError {
    #[error("invalid founder wallet address")]
    InvalidFounderAddress,
    #[error("allocation exceeds total supply")]
    AllocationExceedsSupply,
}

// ============================================================================
// Supply Statistics
// ============================================================================

/// Calculate circulating supply at a given block height
pub fn circulating_supply(block_height: BlockHeight) -> Amount {
    // Initial circulation from genesis allocation
    let genesis_circulation = FOUNDER_IMMEDIATE_AMOUNT;
    
    // Block rewards mined (10 LAT per block)
    let mined_rewards = (block_height as u128) * (BLOCK_REWARD as u128);
    
    // Note: This doesn't account for vesting releases which would add to circulation
    // In practice, you'd track this via state
    
    genesis_circulation + mined_rewards as Amount
}

/// Calculate remaining supply to be mined
pub fn remaining_minable_supply(block_height: BlockHeight) -> Amount {
    let mined = (block_height as u128) * (BLOCK_REWARD as u128);
    let max_minable = TOTAL_SUPPLY - TOTAL_GENESIS_ALLOCATION;
    
    max_minable.saturating_sub(mined as Amount)
}

/// Estimate blocks until max supply is reached
pub fn blocks_until_max_supply() -> u64 {
    let minable_supply = TOTAL_SUPPLY - TOTAL_GENESIS_ALLOCATION;
    (minable_supply / BLOCK_REWARD) as u64
}

// ============================================================================
// Tests
// ============================================================================

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_constants() {
        // Verify total supply is 50 million LAT
        assert_eq!(TOTAL_SUPPLY, 5_000_000_000_000_000); // 50M * 10^8
        
        // Verify block reward is 10 LAT
        assert_eq!(BLOCK_REWARD, 1_000_000_000); // 10 * 10^8
        
        // Verify genesis allocation is 5%
        let percentage = (TOTAL_GENESIS_ALLOCATION * 100) / TOTAL_SUPPLY;
        assert_eq!(percentage, 5);
        
        // Verify immediate amount + vesting = total genesis
        assert_eq!(
            FOUNDER_IMMEDIATE_AMOUNT + FOUNDER_VESTING_AMOUNT,
            TOTAL_GENESIS_ALLOCATION
        );
    }
    
    #[test]
    fn test_lat_conversion() {
        assert_eq!(lat_to_latt(1), 100_000_000);
        assert_eq!(lat_to_latt(10), 1_000_000_000);
        assert_eq!(latt_to_lat(100_000_000), 1);
        assert_eq!(latt_to_lat(1_000_000_000), 10);
    }
    
    #[test]
    fn test_format_lat() {
        assert_eq!(format_lat(100_000_000), "1 LAT");
        assert_eq!(format_lat(1_000_000_000), "10 LAT");
        assert_eq!(format_lat(150_000_000), "1.5 LAT");
        assert_eq!(format_lat(100_000_001), "1.00000001 LAT");
    }
    
    #[test]
    fn test_parse_lat() {
        assert_eq!(parse_lat("1").unwrap(), 100_000_000);
        assert_eq!(parse_lat("1 LAT").unwrap(), 100_000_000);
        assert_eq!(parse_lat("1.5").unwrap(), 150_000_000);
        assert_eq!(parse_lat("10.12345678").unwrap(), 1_012_345_678);
    }
    
    #[test]
    fn test_vesting_schedule() {
        let schedule = VestingSchedule {
            beneficiary: Address::from_bytes([1u8; 20]),
            total_amount: 1000,
            released_amount: 0,
            start_block: 100,
            duration_blocks: 100,
            cliff_blocks: 0,
        };
        
        // Before start
        assert_eq!(schedule.vested_amount(50), 0);
        
        // At start
        assert_eq!(schedule.vested_amount(100), 0);
        
        // 50% through
        assert_eq!(schedule.vested_amount(150), 500);
        
        // 100% through
        assert_eq!(schedule.vested_amount(200), 1000);
        
        // After completion
        assert_eq!(schedule.vested_amount(300), 1000);
    }
    
    #[test]
    fn test_vesting_with_cliff() {
        let schedule = VestingSchedule {
            beneficiary: Address::from_bytes([1u8; 20]),
            total_amount: 1000,
            released_amount: 0,
            start_block: 100,
            duration_blocks: 100,
            cliff_blocks: 25,
        };
        
        // Before cliff
        assert_eq!(schedule.vested_amount(120), 0);
        
        // At cliff
        assert_eq!(schedule.vested_amount(125), 250); // 25% vested at cliff
        
        // After cliff
        assert_eq!(schedule.vested_amount(150), 500);
    }
    
    #[test]
    fn test_founder_address_valid() {
        // Verify the founder address is a valid base58 address
        let result = Address::from_base58(FOUNDER_WALLET_ADDRESS);
        assert!(result.is_ok(), "Founder address should be valid base58");
    }
    
    #[test]
    fn test_genesis_allocations() {
        let allocations = get_genesis_allocations().unwrap();
        
        // Should have one immediate allocation
        assert_eq!(allocations.len(), 1);
        assert_eq!(allocations[0].amount, FOUNDER_IMMEDIATE_AMOUNT);
    }
    
    #[test]
    fn test_blocks_until_max_supply() {
        let blocks = blocks_until_max_supply();
        
        // Minable supply = 50M - 2.5M = 47.5M LAT
        // At 10 LAT per block = 4,750,000 blocks
        assert_eq!(blocks, 4_750_000);
        
        // At 15 sec blocks, this is ~2.26 years
        let years = blocks as f64 / BLOCKS_PER_YEAR as f64;
        assert!(years > 2.0 && years < 2.5);
    }
}
