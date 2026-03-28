//! Difficulty adjustment algorithm
//!
//! Adjusts mining difficulty every 2016 blocks to maintain target block time.
//! Uses a bounded adjustment to prevent extreme difficulty swings.

use lattice_core::{Block, BlockHeight, Timestamp};

/// Target block time in milliseconds (15 seconds)
pub const TARGET_BLOCK_TIME_MS: u64 = 15_000;

/// Number of blocks between difficulty adjustments
pub const ADJUSTMENT_INTERVAL: u64 = 2016;

/// Maximum difficulty adjustment factor (4x)
pub const MAX_ADJUSTMENT_FACTOR: f64 = 4.0;

/// Minimum difficulty adjustment factor (1/4x)
pub const MIN_ADJUSTMENT_FACTOR: f64 = 0.25;

/// Minimum difficulty value
pub const MIN_DIFFICULTY: u64 = 1;

/// Maximum difficulty value (prevent overflow)
pub const MAX_DIFFICULTY: u64 = u64::MAX / 4;

/// Difficulty adjuster that maintains target block time
#[derive(Debug, Clone)]
pub struct DifficultyAdjuster {
    /// Target block time in milliseconds
    target_block_time_ms: u64,
    /// Blocks between adjustments
    adjustment_interval: u64,
    /// Maximum adjustment factor
    max_adjustment: f64,
    /// Minimum adjustment factor
    min_adjustment: f64,
}

impl Default for DifficultyAdjuster {
    fn default() -> Self {
        Self {
            target_block_time_ms: TARGET_BLOCK_TIME_MS,
            adjustment_interval: ADJUSTMENT_INTERVAL,
            max_adjustment: MAX_ADJUSTMENT_FACTOR,
            min_adjustment: MIN_ADJUSTMENT_FACTOR,
        }
    }
}

impl DifficultyAdjuster {
    /// Create a new difficulty adjuster with default parameters
    pub fn new() -> Self {
        Self::default()
    }

    /// Create a difficulty adjuster with custom parameters
    pub fn with_params(
        target_block_time_ms: u64,
        adjustment_interval: u64,
        max_adjustment: f64,
    ) -> Self {
        Self {
            target_block_time_ms,
            adjustment_interval,
            max_adjustment,
            min_adjustment: 1.0 / max_adjustment,
        }
    }

    /// Check if difficulty should be adjusted at this height
    pub fn should_adjust(&self, height: BlockHeight) -> bool {
        height != 0 && height.is_multiple_of(self.adjustment_interval)
    }

    /// Calculate the next difficulty based on recent block times
    ///
    /// `current_difficulty` - The current difficulty
    /// `interval_start_time` - Timestamp of the first block in the interval
    /// `interval_end_time` - Timestamp of the last block in the interval
    pub fn calculate_next_difficulty(
        &self,
        current_difficulty: u64,
        interval_start_time: Timestamp,
        interval_end_time: Timestamp,
    ) -> u64 {
        // Calculate actual time taken for the interval
        let actual_time = interval_end_time.saturating_sub(interval_start_time);

        // Expected time for the interval
        let expected_time = self.target_block_time_ms * self.adjustment_interval;

        // Calculate adjustment ratio
        let mut ratio = expected_time as f64 / actual_time.max(1) as f64;

        // Clamp to bounds
        ratio = ratio.clamp(self.min_adjustment, self.max_adjustment);

        // Calculate new difficulty
        let new_difficulty = (current_difficulty as f64 * ratio).round() as u64;

        // Clamp to valid range
        new_difficulty.clamp(MIN_DIFFICULTY, MAX_DIFFICULTY)
    }

    /// Calculate next difficulty from a list of blocks in the adjustment interval
    pub fn adjust_from_blocks(&self, blocks: &[Block]) -> Option<u64> {
        if blocks.len() < 2 {
            return None;
        }

        let first = blocks.first()?;
        let last = blocks.last()?;

        // Ensure this is a valid adjustment interval
        if !self.should_adjust(last.height()) {
            return None;
        }

        let current_difficulty = last.header.difficulty;
        let start_time = first.header.timestamp;
        let end_time = last.header.timestamp;

        Some(self.calculate_next_difficulty(
            current_difficulty,
            start_time,
            end_time,
        ))
    }

    /// Calculate the average block time over a set of blocks
    pub fn average_block_time(&self, blocks: &[Block]) -> Option<u64> {
        if blocks.len() < 2 {
            return None;
        }

        let first_time = blocks.first()?.header.timestamp;
        let last_time = blocks.last()?.header.timestamp;
        let total_time = last_time.saturating_sub(first_time);
        let block_count = (blocks.len() - 1) as u64;

        Some(total_time / block_count.max(1))
    }

    /// Estimate time to find next block at given difficulty and hash rate
    pub fn estimate_block_time(&self, difficulty: u64, hash_rate: u64) -> u64 {
        if hash_rate == 0 {
            return u64::MAX;
        }

        // Expected hashes = 2^(log2(difficulty))
        let expected_hashes = difficulty;
        (expected_hashes * 1000) / hash_rate // milliseconds
    }

    /// Get the adjustment interval
    pub fn adjustment_interval(&self) -> u64 {
        self.adjustment_interval
    }

    /// Get the target block time
    pub fn target_block_time_ms(&self) -> u64 {
        self.target_block_time_ms
    }
}

/// Statistics about difficulty over a range of blocks
#[derive(Debug, Clone)]
pub struct DifficultyStats {
    /// Minimum difficulty in the range
    pub min: u64,
    /// Maximum difficulty in the range
    pub max: u64,
    /// Average difficulty
    pub average: f64,
    /// Current difficulty (latest block)
    pub current: u64,
}

impl DifficultyStats {
    /// Calculate statistics from a list of blocks
    pub fn from_blocks(blocks: &[Block]) -> Option<Self> {
        if blocks.is_empty() {
            return None;
        }

        let difficulties: Vec<u64> = blocks.iter().map(|b| b.header.difficulty).collect();

        let min = *difficulties.iter().min()?;
        let max = *difficulties.iter().max()?;
        let sum: u64 = difficulties.iter().sum();
        let average = sum as f64 / difficulties.len() as f64;
        let current = blocks.last()?.header.difficulty;

        Some(Self {
            min,
            max,
            average,
            current,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::{Address, BlockHeader};

    fn create_test_block(height: BlockHeight, timestamp: Timestamp, difficulty: u64) -> Block {
        let header = BlockHeader {
            version: 1,
            height,
            prev_hash: [0u8; 32],
            tx_root: [0u8; 32],
            state_root: [0u8; 32],
            timestamp,
            difficulty,
            nonce: 0,
            coinbase: Address::zero(),
        };
        Block::new(header, vec![])
    }

    #[test]
    fn test_should_adjust() {
        let adjuster = DifficultyAdjuster::new();

        assert!(!adjuster.should_adjust(0)); // Genesis
        assert!(!adjuster.should_adjust(1));
        assert!(!adjuster.should_adjust(2015));
        assert!(adjuster.should_adjust(2016));
        assert!(!adjuster.should_adjust(2017));
        assert!(adjuster.should_adjust(4032));
    }

    #[test]
    fn test_difficulty_increase_when_fast() {
        let adjuster = DifficultyAdjuster::new();

        // Blocks mined too fast (half expected time)
        let expected_time = TARGET_BLOCK_TIME_MS * ADJUSTMENT_INTERVAL;
        let actual_time = expected_time / 2; // Twice as fast

        let new_difficulty =
            adjuster.calculate_next_difficulty(1000, 0, actual_time);

        // Difficulty should increase (capped at 4x)
        assert!(new_difficulty > 1000);
        assert!(new_difficulty <= 4000); // Max 4x increase
    }

    #[test]
    fn test_difficulty_decrease_when_slow() {
        let adjuster = DifficultyAdjuster::new();

        // Blocks mined too slow (double expected time)
        let expected_time = TARGET_BLOCK_TIME_MS * ADJUSTMENT_INTERVAL;
        let actual_time = expected_time * 2; // Half as fast

        let new_difficulty =
            adjuster.calculate_next_difficulty(1000, 0, actual_time);

        // Difficulty should decrease (min 1/4x)
        assert!(new_difficulty < 1000);
        assert!(new_difficulty >= 250); // Max 4x decrease
    }

    #[test]
    fn test_difficulty_stable_at_target() {
        let adjuster = DifficultyAdjuster::new();

        // Blocks mined at exactly target time
        let expected_time = TARGET_BLOCK_TIME_MS * ADJUSTMENT_INTERVAL;

        let new_difficulty =
            adjuster.calculate_next_difficulty(1000, 0, expected_time);

        // Difficulty should stay the same (or very close)
        assert!((new_difficulty as i64 - 1000).abs() <= 1);
    }

    #[test]
    fn test_max_adjustment_clamp() {
        let adjuster = DifficultyAdjuster::new();

        // Blocks mined extremely fast (should be capped at 4x)
        let new_difficulty = adjuster.calculate_next_difficulty(1000, 0, 1); // Nearly instant

        assert_eq!(new_difficulty, 4000); // Capped at 4x
    }

    #[test]
    fn test_min_adjustment_clamp() {
        let adjuster = DifficultyAdjuster::new();

        // Blocks mined extremely slow (should be capped at 1/4x)
        let expected_time = TARGET_BLOCK_TIME_MS * ADJUSTMENT_INTERVAL;
        let new_difficulty =
            adjuster.calculate_next_difficulty(1000, 0, expected_time * 100);

        assert_eq!(new_difficulty, 250); // Capped at 1/4x
    }

    #[test]
    fn test_min_difficulty_bound() {
        let adjuster = DifficultyAdjuster::new();

        // Very slow mining with low difficulty
        let expected_time = TARGET_BLOCK_TIME_MS * ADJUSTMENT_INTERVAL;
        let new_difficulty =
            adjuster.calculate_next_difficulty(1, 0, expected_time * 100);

        assert!(new_difficulty >= MIN_DIFFICULTY);
    }

    #[test]
    fn test_adjust_from_blocks() {
        let adjuster = DifficultyAdjuster::with_params(15_000, 10, 4.0);

        // Create blocks at exactly target time
        let blocks: Vec<Block> = (0..=10)
            .map(|i| create_test_block(i, i * 15_000, 1000))
            .collect();

        let new_diff = adjuster.adjust_from_blocks(&blocks).unwrap();

        // Should stay approximately the same
        assert!((new_diff as i64 - 1000).abs() <= 1);
    }

    #[test]
    fn test_average_block_time() {
        let adjuster = DifficultyAdjuster::new();

        let blocks: Vec<Block> = (0..5)
            .map(|i| create_test_block(i, i * 20_000, 1000)) // 20 second blocks
            .collect();

        let avg = adjuster.average_block_time(&blocks).unwrap();
        assert_eq!(avg, 20_000);
    }

    #[test]
    fn test_difficulty_stats() {
        let blocks = vec![
            create_test_block(0, 0, 100),
            create_test_block(1, 15000, 150),
            create_test_block(2, 30000, 200),
        ];

        let stats = DifficultyStats::from_blocks(&blocks).unwrap();

        assert_eq!(stats.min, 100);
        assert_eq!(stats.max, 200);
        assert_eq!(stats.current, 200);
        assert!((stats.average - 150.0).abs() < 0.01);
    }
}
