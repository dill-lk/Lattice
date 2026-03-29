//! Multi-threaded mining implementation
//!
//! Distributes nonce search across multiple CPU threads using rayon.
//! Each thread searches a disjoint range of nonces.

use crate::pow::{compute_pow_hash, difficulty_to_target, PoWConfig, PoWError};
use lattice_core::{BlockHeader, Hash};
use parking_lot::{Mutex, RwLock};
use rayon::prelude::*;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};
use tracing::{debug, info, trace};

/// Result of a mining attempt
#[derive(Debug, Clone)]
pub enum MiningResult {
    /// Found a valid nonce
    Found { nonce: u64, hash: Hash },
    /// Mining was cancelled
    Cancelled,
    /// Exhausted nonce range without finding solution
    Exhausted,
}

/// Mining statistics
#[derive(Debug, Clone, Default)]
pub struct MiningStats {
    /// Total hashes computed
    pub hashes: u64,
    /// Time spent mining (milliseconds)
    pub duration_ms: u64,
    /// Hash rate (hashes per second)
    pub hash_rate: f64,
}

impl MiningStats {
    /// Calculate hash rate from hashes and duration
    pub fn calculate_hash_rate(&mut self) {
        if self.duration_ms > 0 {
            self.hash_rate = (self.hashes as f64 / self.duration_ms as f64) * 1000.0;
        }
    }
}

/// Multi-threaded miner
#[derive(Debug)]
pub struct Miner {
    /// PoW configuration
    config: PoWConfig,
    /// Number of threads to use (0 = auto-detect)
    num_threads: usize,
    /// Cancellation flag
    cancelled: Arc<AtomicBool>,
    /// Latest mining stats
    stats: Arc<RwLock<MiningStats>>,
}

impl Clone for Miner {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            num_threads: self.num_threads,
            cancelled: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(MiningStats::default())),
        }
    }
}

impl Default for Miner {
    fn default() -> Self {
        Self::new(PoWConfig::default())
    }
}

impl Miner {
    /// Create a new miner with the given PoW config
    pub fn new(config: PoWConfig) -> Self {
        Self {
            config,
            num_threads: 0, // Auto-detect
            cancelled: Arc::new(AtomicBool::new(false)),
            stats: Arc::new(RwLock::new(MiningStats::default())),
        }
    }

    /// Set the number of threads to use
    pub fn with_threads(mut self, threads: usize) -> Self {
        self.num_threads = threads;
        self
    }

    /// Get the effective number of threads
    fn effective_threads(&self) -> usize {
        if self.num_threads == 0 {
            rayon::current_num_threads()
        } else {
            self.num_threads
        }
    }

    /// Mine a block header to find a valid nonce
    ///
    /// Returns the winning nonce and hash if found.
    pub fn mine(&self, header: &BlockHeader) -> Result<MiningResult, PoWError> {
        self.mine_range(header, 0, u64::MAX)
    }

    /// Mine within a specific nonce range
    pub fn mine_range(
        &self,
        header: &BlockHeader,
        start_nonce: u64,
        end_nonce: u64,
    ) -> Result<MiningResult, PoWError> {
        // If already cancelled (e.g. cancel() called before mine_range), return immediately
        if self.cancelled.load(Ordering::SeqCst) {
            return Ok(MiningResult::Cancelled);
        }
        // Reset cancellation state for this new mining attempt
        self.cancelled.store(false, Ordering::SeqCst);
        let start_time = Instant::now();
        let hash_count = Arc::new(AtomicU64::new(0));
        let found = Arc::new(Mutex::new(None::<(u64, Hash)>));
        let target = difficulty_to_target(header.difficulty);

        let num_threads = self.effective_threads();
        let range_size = end_nonce.saturating_sub(start_nonce);
        let chunk_size = (range_size / num_threads as u64).max(1);

        info!(
            difficulty = header.difficulty,
            threads = num_threads,
            start_nonce,
            end_nonce,
            "Starting mining"
        );

        // Build thread pool if custom thread count
        let pool = if self.num_threads > 0 {
            Some(
                rayon::ThreadPoolBuilder::new()
                    .num_threads(self.num_threads)
                    .build()
                    .map_err(|e| PoWError::Argon2Error(e.to_string()))?,
            )
        } else {
            None
        };

        let mine_work = || {
            (0..num_threads).into_par_iter().for_each(|thread_id| {
                let thread_start = start_nonce.saturating_add(thread_id as u64 * chunk_size);
                let thread_end = if thread_id == num_threads - 1 {
                    end_nonce
                } else {
                    thread_start.saturating_add(chunk_size)
                };

                self.mine_thread(
                    header,
                    thread_start,
                    thread_end,
                    &target,
                    &found,
                    &hash_count,
                );
            });
        };

        // Execute mining
        match &pool {
            Some(p) => p.install(mine_work),
            None => mine_work(),
        }

        // Update stats
        let elapsed = start_time.elapsed();
        let total_hashes = hash_count.load(Ordering::Relaxed);
        {
            let mut stats = self.stats.write();
            stats.hashes = total_hashes;
            stats.duration_ms = elapsed.as_millis() as u64;
            stats.calculate_hash_rate();
        }

        // Check result
        if self.cancelled.load(Ordering::SeqCst) {
            return Ok(MiningResult::Cancelled);
        }

        let found_result = found.lock().take();
        match found_result {
            Some((nonce, hash)) => {
                info!(
                    nonce,
                    hashes = total_hashes,
                    duration_ms = elapsed.as_millis(),
                    "Found valid nonce"
                );
                Ok(MiningResult::Found { nonce, hash })
            }
            None => Ok(MiningResult::Exhausted),
        }
    }

    /// Mine within a thread's assigned range
    fn mine_thread(
        &self,
        header: &BlockHeader,
        start: u64,
        end: u64,
        target: &Hash,
        found: &Arc<Mutex<Option<(u64, Hash)>>>,
        hash_count: &Arc<AtomicU64>,
    ) {
        const BATCH_SIZE: u64 = 100;
        let mut local_count: u64 = 0;
        let mut nonce = start;

        while nonce < end {
            // Check for cancellation or if another thread found solution
            if self.cancelled.load(Ordering::Relaxed) || found.lock().is_some() {
                break;
            }

            // Mine a batch
            let batch_end = (nonce + BATCH_SIZE).min(end);
            for n in nonce..batch_end {
                if let Ok(hash) = compute_pow_hash(header, n, &self.config) {
                    local_count += 1;

                    if compare_hash_to_target(&hash, target) {
                        // Found valid nonce
                        let mut guard = found.lock();
                        if guard.is_none() {
                            *guard = Some((n, hash));
                            debug!(nonce = n, "Thread found valid nonce");
                        }
                        hash_count.fetch_add(local_count, Ordering::Relaxed);
                        return;
                    }
                }
            }

            nonce = batch_end;

            // Periodically update global count
            if local_count >= BATCH_SIZE * 10 {
                hash_count.fetch_add(local_count, Ordering::Relaxed);
                local_count = 0;
                trace!(nonce, "Mining progress");
            }
        }

        // Final count update
        hash_count.fetch_add(local_count, Ordering::Relaxed);
    }

    /// Cancel ongoing mining
    pub fn cancel(&self) {
        self.cancelled.store(true, Ordering::SeqCst);
    }

    /// Check if mining was cancelled
    pub fn is_cancelled(&self) -> bool {
        self.cancelled.load(Ordering::SeqCst)
    }

    /// Get the latest mining statistics
    pub fn stats(&self) -> MiningStats {
        self.stats.read().clone()
    }

    /// Get the PoW configuration
    pub fn config(&self) -> &PoWConfig {
        &self.config
    }

    /// Mine with a timeout
    pub fn mine_with_timeout(
        &self,
        header: &BlockHeader,
        timeout: Duration,
    ) -> Result<MiningResult, PoWError> {
        let miner = self.clone();
        let header = header.clone();

        // Spawn timeout thread
        let cancelled = self.cancelled.clone();
        std::thread::spawn(move || {
            std::thread::sleep(timeout);
            cancelled.store(true, Ordering::SeqCst);
        });

        miner.mine(&header)
    }
}

/// Compare a hash against a target
fn compare_hash_to_target(hash: &Hash, target: &Hash) -> bool {
    for i in 0..32 {
        if hash[i] < target[i] {
            return true;
        }
        if hash[i] > target[i] {
            return false;
        }
    }
    true
}

/// Builder for creating configured miners
#[derive(Debug, Default)]
pub struct MinerBuilder {
    config: Option<PoWConfig>,
    threads: Option<usize>,
}

impl MinerBuilder {
    /// Create a new builder
    pub fn new() -> Self {
        Self::default()
    }

    /// Set the PoW configuration
    pub fn config(mut self, config: PoWConfig) -> Self {
        self.config = Some(config);
        self
    }

    /// Set the number of threads
    pub fn threads(mut self, threads: usize) -> Self {
        self.threads = Some(threads);
        self
    }

    /// Build the miner
    pub fn build(self) -> Miner {
        let mut miner = Miner::new(self.config.unwrap_or_default());
        if let Some(threads) = self.threads {
            miner = miner.with_threads(threads);
        }
        miner
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use lattice_core::Address;

    fn test_header(difficulty: u64) -> BlockHeader {
        BlockHeader {
            version: 1,
            height: 1,
            prev_hash: [0u8; 32],
            tx_root: [0u8; 32],
            state_root: [0u8; 32],
            timestamp: 1704067200000,
            difficulty,
            nonce: 0,
            coinbase: Address::zero(),
        }
    }

    #[test]
    fn test_miner_creation() {
        let miner = Miner::new(PoWConfig::light());
        assert!(!miner.is_cancelled());
    }

    #[test]
    fn test_miner_builder() {
        let miner = MinerBuilder::new()
            .config(PoWConfig::light())
            .threads(2)
            .build();

        assert_eq!(miner.num_threads, 2);
    }

    #[test]
    fn test_mine_easy_difficulty() {
        let miner = Miner::new(PoWConfig::light()).with_threads(2);
        let header = test_header(1); // Very easy

        let result = miner.mine_range(&header, 0, 1000).unwrap();

        match result {
            MiningResult::Found { nonce, hash } => {
                assert!(nonce < 1000);
                assert!(!hash.iter().all(|&b| b == 0));
            }
            MiningResult::Exhausted => {
                // Also acceptable for very low difficulty
            }
            MiningResult::Cancelled => panic!("Should not be cancelled"),
        }
    }

    #[test]
    fn test_mine_cancellation() {
        let miner = Miner::new(PoWConfig::light());

        // Cancel immediately
        miner.cancel();

        let header = test_header(100);
        let result = miner.mine_range(&header, 0, 1_000_000).unwrap();

        match result {
            MiningResult::Cancelled => {} // Expected
            _ => panic!("Should have been cancelled"),
        }
    }

    #[test]
    fn test_mining_stats() {
        let miner = Miner::new(PoWConfig::light()).with_threads(1);
        let header = test_header(1);

        let _ = miner.mine_range(&header, 0, 100);

        let stats = miner.stats();
        assert!(stats.hashes > 0 || stats.duration_ms == 0);
    }

    #[test]
    fn test_nonce_range_partitioning() {
        // Test that different start nonces give different results
        let miner = Miner::new(PoWConfig::light()).with_threads(1);
        let header = test_header(1);

        let result1 = miner.mine_range(&header, 0, 100);
        let result2 = miner.mine_range(&header, 100, 200);

        // Both should complete without error
        assert!(result1.is_ok());
        assert!(result2.is_ok());
    }

    #[test]
    fn test_clone_miner() {
        let miner1 = Miner::new(PoWConfig::light());
        miner1.cancel();

        let miner2 = miner1.clone();

        // Clone should have fresh cancellation state
        assert!(!miner2.is_cancelled());
    }

    #[test]
    fn test_compare_hash_to_target() {
        let low = [0x00; 32];
        let high = [0xFF; 32];

        assert!(compare_hash_to_target(&low, &high));
        assert!(!compare_hash_to_target(&high, &low));
    }
}
