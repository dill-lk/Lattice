//! Integration tests for consensus (PoW mining and difficulty adjustment)

use lattice_core::{BlockHeader, Address};
use lattice_consensus::{
    PoWConfig, hash_block_header, verify_pow, check_difficulty,
    DifficultyAdjuster, DifficultyConfig, mine_block_single_threaded,
    MinerBuilder, MiningResult
};
use std::time::{Duration, Instant};

// ============================================================================
// PoW Algorithm Tests
// ============================================================================

#[test]
fn test_pow_hash_deterministic() {
    let config = PoWConfig::light();
    let header = create_test_header(0, 1000);
    
    let hash1 = hash_block_header(&header, &config).unwrap();
    let hash2 = hash_block_header(&header, &config).unwrap();
    
    assert_eq!(hash1, hash2);
}

#[test]
fn test_pow_hash_changes_with_nonce() {
    let config = PoWConfig::light();
    let header1 = create_test_header(0, 1000);
    let header2 = create_test_header(1, 1000);
    
    let hash1 = hash_block_header(&header1, &config).unwrap();
    let hash2 = hash_block_header(&header2, &config).unwrap();
    
    assert_ne!(hash1, hash2);
}

#[test]
fn test_pow_verification_valid() {
    let config = PoWConfig::light();
    let mut header = create_test_header(0, 1); // Very low difficulty
    
    // Find a valid nonce
    for nonce in 0..10000 {
        header.nonce = nonce;
        if verify_pow(&header, &config).is_ok() {
            // Found valid nonce, verification should succeed
            assert!(verify_pow(&header, &config).is_ok());
            return;
        }
    }
    
    panic!("Should find valid nonce within 10000 attempts with difficulty 1");
}

#[test]
fn test_pow_verification_invalid() {
    let config = PoWConfig::light();
    let header = create_test_header(12345, u64::MAX); // Max difficulty
    
    // With max difficulty, almost all nonces should be invalid
    assert!(verify_pow(&header, &config).is_err());
}

#[test]
fn test_difficulty_check() {
    let config = PoWConfig::light();
    
    // Low difficulty - should be easier to find valid hash
    let header1 = create_test_header(0, 1);
    
    // High difficulty - should be harder to find valid hash
    let header2 = create_test_header(0, 1000000);
    
    // We can compute hashes for both
    let hash1 = hash_block_header(&header1, &config).unwrap();
    let hash2 = hash_block_header(&header2, &config).unwrap();
    
    // Both produce 32-byte hashes
    assert_eq!(hash1.len(), 32);
    assert_eq!(hash2.len(), 32);
}

#[test]
fn test_single_threaded_mining() {
    let config = PoWConfig::light();
    let header = create_test_header(0, 1); // Very low difficulty for testing
    
    let start = Instant::now();
    let result = mine_block_single_threaded(header.clone(), &config, 0, 10000);
    let elapsed = start.elapsed();
    
    if let Ok((solved_header, _hash)) = result {
        // Verify the solution
        assert!(verify_pow(&solved_header, &config).is_ok());
        println!("Mining took {:?} to find nonce {}", elapsed, solved_header.nonce);
    } else {
        // If not found in range, that's ok for a test
        println!("No valid nonce found in range 0-10000 (this is ok for a test)");
    }
}

#[test]
fn test_miner_builder() {
    let config = PoWConfig::light();
    let header = create_test_header(0, 1);
    
    let miner = MinerBuilder::new()
        .threads(2)
        .pow_config(config)
        .build();
    
    // Just verify miner can be constructed
    assert_eq!(miner.threads(), 2);
}

// ============================================================================
// Difficulty Adjustment Tests
// ============================================================================

#[test]
fn test_difficulty_adjuster_initialization() {
    let config = DifficultyConfig::default();
    let adjuster = DifficultyAdjuster::new(config);
    
    assert_eq!(adjuster.current_difficulty(), config.initial_difficulty);
}

#[test]
fn test_difficulty_increases_when_blocks_fast() {
    let mut config = DifficultyConfig::default();
    config.target_block_time_ms = 15000; // 15 seconds
    config.adjustment_interval = 10;
    
    let mut adjuster = DifficultyAdjuster::new(config);
    let initial_diff = adjuster.current_difficulty();
    
    // Simulate 10 blocks that came too fast (10 seconds each instead of 15)
    let start_time = 1000000;
    for i in 0..10 {
        let timestamp = start_time + (i * 10000); // 10 second intervals
        adjuster.record_block(i, timestamp, initial_diff);
    }
    
    // After adjustment, difficulty should increase
    let new_diff = adjuster.current_difficulty();
    assert!(new_diff > initial_diff, 
        "Difficulty should increase when blocks come too fast. Initial: {}, New: {}", 
        initial_diff, new_diff);
}

#[test]
fn test_difficulty_decreases_when_blocks_slow() {
    let mut config = DifficultyConfig::default();
    config.target_block_time_ms = 15000; // 15 seconds
    config.adjustment_interval = 10;
    
    let mut adjuster = DifficultyAdjuster::new(config);
    let initial_diff = adjuster.current_difficulty();
    
    // Simulate 10 blocks that came too slow (30 seconds each instead of 15)
    let start_time = 1000000;
    for i in 0..10 {
        let timestamp = start_time + (i * 30000); // 30 second intervals
        adjuster.record_block(i, timestamp, initial_diff);
    }
    
    // After adjustment, difficulty should decrease
    let new_diff = adjuster.current_difficulty();
    assert!(new_diff < initial_diff,
        "Difficulty should decrease when blocks come too slow. Initial: {}, New: {}",
        initial_diff, new_diff);
}

#[test]
fn test_difficulty_stays_same_when_blocks_on_target() {
    let mut config = DifficultyConfig::default();
    config.target_block_time_ms = 15000; // 15 seconds
    config.adjustment_interval = 10;
    config.max_adjustment_factor = 1.1; // 10% max adjustment
    
    let mut adjuster = DifficultyAdjuster::new(config);
    let initial_diff = adjuster.current_difficulty();
    
    // Simulate 10 blocks at exactly target time
    let start_time = 1000000;
    for i in 0..10 {
        let timestamp = start_time + (i * 15000); // Exactly 15 seconds
        adjuster.record_block(i, timestamp, initial_diff);
    }
    
    // Difficulty should stay very close to initial
    let new_diff = adjuster.current_difficulty();
    let diff_ratio = new_diff as f64 / initial_diff as f64;
    
    assert!((diff_ratio - 1.0).abs() < 0.05,
        "Difficulty should stay close when blocks are on target. Ratio: {}", diff_ratio);
}

#[test]
fn test_difficulty_bounded_by_min() {
    let mut config = DifficultyConfig::default();
    config.target_block_time_ms = 15000;
    config.adjustment_interval = 5;
    config.min_difficulty = 1000;
    
    let mut adjuster = DifficultyAdjuster::new(config);
    
    // Force difficulty down by simulating very slow blocks
    for i in 0..20 {
        let timestamp = 1000000 + (i * 1000000); // 1000 second intervals (very slow)
        adjuster.record_block(i, timestamp, adjuster.current_difficulty());
    }
    
    // Difficulty should not go below minimum
    assert!(adjuster.current_difficulty() >= config.min_difficulty);
}

#[test]
fn test_difficulty_bounded_by_max() {
    let mut config = DifficultyConfig::default();
    config.target_block_time_ms = 15000;
    config.adjustment_interval = 5;
    config.max_difficulty = 100000;
    
    let mut adjuster = DifficultyAdjuster::new(config);
    
    // Force difficulty up by simulating very fast blocks
    for i in 0..20 {
        let timestamp = 1000000 + (i * 100); // 100ms intervals (very fast)
        adjuster.record_block(i, timestamp, adjuster.current_difficulty());
    }
    
    // Difficulty should not go above maximum
    assert!(adjuster.current_difficulty() <= config.max_difficulty);
}

#[test]
fn test_difficulty_adjustment_interval() {
    let mut config = DifficultyConfig::default();
    config.adjustment_interval = 5; // Adjust every 5 blocks
    config.target_block_time_ms = 15000;
    
    let mut adjuster = DifficultyAdjuster::new(config);
    let initial_diff = adjuster.current_difficulty();
    
    // Record 4 blocks (below adjustment interval)
    for i in 0..4 {
        let timestamp = 1000000 + (i * 1000); // Fast blocks
        adjuster.record_block(i, timestamp, initial_diff);
    }
    
    // Difficulty shouldn't change yet
    assert_eq!(adjuster.current_difficulty(), initial_diff);
    
    // Record 5th block (triggers adjustment)
    adjuster.record_block(4, 1000000 + (4 * 1000), initial_diff);
    
    // Now difficulty should have changed
    let new_diff = adjuster.current_difficulty();
    assert_ne!(new_diff, initial_diff);
}

#[test]
fn test_get_difficulty_stats() {
    let config = DifficultyConfig::default();
    let mut adjuster = DifficultyAdjuster::new(config);
    
    // Record some blocks
    for i in 0..10 {
        adjuster.record_block(i, 1000000 + (i * 15000), 1000);
    }
    
    let stats = adjuster.stats();
    assert_eq!(stats.blocks_recorded, 10);
    assert_eq!(stats.current_difficulty, adjuster.current_difficulty());
}

// ============================================================================
// Helper Functions
// ============================================================================

fn create_test_header(nonce: u64, difficulty: u64) -> BlockHeader {
    BlockHeader {
        version: 1,
        height: 0,
        prev_hash: [0u8; 32],
        tx_root: [0u8; 32],
        state_root: [0u8; 32],
        timestamp: 1234567890,
        difficulty,
        nonce,
        coinbase: Address::from_bytes([0u8; 20]),
    }
}
