//! Lattice Consensus - Proof of Work consensus engine
//!
//! Memory-hard, CPU-friendly mining algorithm designed to be ASIC-resistant.
//! Uses Argon2 for memory-hard hashing with multi-threaded nonce search.

mod difficulty;
mod miner;
mod pow;

pub use difficulty::{
    DifficultyAdjuster, DifficultyStats, ADJUSTMENT_INTERVAL, MAX_ADJUSTMENT_FACTOR,
    MAX_DIFFICULTY, MIN_ADJUSTMENT_FACTOR, MIN_DIFFICULTY, TARGET_BLOCK_TIME_MS,
};
pub use miner::{Miner, MinerBuilder, MiningResult, MiningStats};
pub use pow::{
    compute_pow_hash, difficulty_to_target, estimated_hashes_for_difficulty, verify_pow,
    PoWConfig, PoWError,
};
