//! Mining statistics tracking

use parking_lot::RwLock;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use std::time::{Duration, Instant};

/// Mining statistics tracker
#[derive(Debug)]
pub struct MiningStats {
    /// Time when mining started
    start_time: Instant,
    /// Total hashes computed
    total_hashes: AtomicU64,
    /// Blocks found
    blocks_found: AtomicU64,
    /// Blocks submitted but rejected
    blocks_rejected: AtomicU64,
    /// Last hash rate calculation
    last_hash_rate: RwLock<HashRateSample>,
    /// Hash rate history for averaging
    hash_rate_history: RwLock<Vec<f64>>,
}

#[derive(Debug, Clone)]
struct HashRateSample {
    hashes: u64,
    timestamp: Instant,
    rate: f64,
}

impl Default for HashRateSample {
    fn default() -> Self {
        Self {
            hashes: 0,
            timestamp: Instant::now(),
            rate: 0.0,
        }
    }
}

impl MiningStats {
    /// Create a new stats tracker
    pub fn new() -> Arc<Self> {
        Arc::new(Self {
            start_time: Instant::now(),
            total_hashes: AtomicU64::new(0),
            blocks_found: AtomicU64::new(0),
            blocks_rejected: AtomicU64::new(0),
            last_hash_rate: RwLock::new(HashRateSample::default()),
            hash_rate_history: RwLock::new(Vec::with_capacity(60)),
        })
    }

    /// Add hashes to the total count
    pub fn add_hashes(&self, count: u64) {
        self.total_hashes.fetch_add(count, Ordering::Relaxed);
    }

    /// Record a found block
    pub fn record_block_found(&self) {
        self.blocks_found.fetch_add(1, Ordering::Relaxed);
    }

    /// Record a rejected block
    pub fn record_block_rejected(&self) {
        self.blocks_rejected.fetch_add(1, Ordering::Relaxed);
    }

    /// Get total hashes computed
    pub fn total_hashes(&self) -> u64 {
        self.total_hashes.load(Ordering::Relaxed)
    }

    /// Get blocks found count
    pub fn blocks_found(&self) -> u64 {
        self.blocks_found.load(Ordering::Relaxed)
    }

    /// Get blocks rejected count
    pub fn blocks_rejected(&self) -> u64 {
        self.blocks_rejected.load(Ordering::Relaxed)
    }

    /// Get uptime duration
    pub fn uptime(&self) -> Duration {
        self.start_time.elapsed()
    }

    /// Get formatted uptime string
    pub fn uptime_string(&self) -> String {
        let duration = self.uptime();
        let secs = duration.as_secs();
        let hours = secs / 3600;
        let mins = (secs % 3600) / 60;
        let secs = secs % 60;

        if hours > 0 {
            format!("{}h {}m {}s", hours, mins, secs)
        } else if mins > 0 {
            format!("{}m {}s", mins, secs)
        } else {
            format!("{}s", secs)
        }
    }

    /// Calculate current hash rate (hashes per second)
    pub fn calculate_hash_rate(&self) -> f64 {
        let current_hashes = self.total_hashes();
        let now = Instant::now();

        let mut last = self.last_hash_rate.write();
        let elapsed = now.duration_since(last.timestamp).as_secs_f64();

        // Only recalculate if at least 1 second has passed
        if elapsed >= 1.0 {
            let hash_diff = current_hashes.saturating_sub(last.hashes);
            let rate = hash_diff as f64 / elapsed;

            last.hashes = current_hashes;
            last.timestamp = now;
            last.rate = rate;

            // Add to history for averaging
            let mut history = self.hash_rate_history.write();
            history.push(rate);
            if history.len() > 60 {
                history.remove(0);
            }
        }

        last.rate
    }

    /// Get average hash rate over the history window
    pub fn average_hash_rate(&self) -> f64 {
        let history = self.hash_rate_history.read();
        if history.is_empty() {
            return 0.0;
        }
        history.iter().sum::<f64>() / history.len() as f64
    }

    /// Format hash rate for display
    pub fn format_hash_rate(rate: f64) -> String {
        if rate >= 1_000_000_000.0 {
            format!("{:.2} GH/s", rate / 1_000_000_000.0)
        } else if rate >= 1_000_000.0 {
            format!("{:.2} MH/s", rate / 1_000_000.0)
        } else if rate >= 1_000.0 {
            format!("{:.2} KH/s", rate / 1_000.0)
        } else {
            format!("{:.2} H/s", rate)
        }
    }

    /// Get a snapshot of current stats
    pub fn snapshot(&self) -> StatsSnapshot {
        StatsSnapshot {
            uptime: self.uptime(),
            total_hashes: self.total_hashes(),
            blocks_found: self.blocks_found(),
            blocks_rejected: self.blocks_rejected(),
            current_hash_rate: self.calculate_hash_rate(),
            average_hash_rate: self.average_hash_rate(),
        }
    }
}

impl Default for MiningStats {
    fn default() -> Self {
        Self {
            start_time: Instant::now(),
            total_hashes: AtomicU64::new(0),
            blocks_found: AtomicU64::new(0),
            blocks_rejected: AtomicU64::new(0),
            last_hash_rate: RwLock::new(HashRateSample::default()),
            hash_rate_history: RwLock::new(Vec::with_capacity(60)),
        }
    }
}

/// Snapshot of mining statistics
#[derive(Debug, Clone)]
pub struct StatsSnapshot {
    pub uptime: Duration,
    pub total_hashes: u64,
    pub blocks_found: u64,
    pub blocks_rejected: u64,
    pub current_hash_rate: f64,
    pub average_hash_rate: f64,
}

impl std::fmt::Display for StatsSnapshot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let uptime_secs = self.uptime.as_secs();
        let hours = uptime_secs / 3600;
        let mins = (uptime_secs % 3600) / 60;
        let secs = uptime_secs % 60;

        write!(
            f,
            "Uptime: {:02}:{:02}:{:02} | Hashrate: {} (avg: {}) | Blocks: {} found, {} rejected",
            hours,
            mins,
            secs,
            MiningStats::format_hash_rate(self.current_hash_rate),
            MiningStats::format_hash_rate(self.average_hash_rate),
            self.blocks_found,
            self.blocks_rejected
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_stats_creation() {
        let stats = MiningStats::new();
        assert_eq!(stats.total_hashes(), 0);
        assert_eq!(stats.blocks_found(), 0);
    }

    #[test]
    fn test_add_hashes() {
        let stats = MiningStats::new();
        stats.add_hashes(100);
        stats.add_hashes(50);
        assert_eq!(stats.total_hashes(), 150);
    }

    #[test]
    fn test_block_counting() {
        let stats = MiningStats::new();
        stats.record_block_found();
        stats.record_block_found();
        stats.record_block_rejected();
        assert_eq!(stats.blocks_found(), 2);
        assert_eq!(stats.blocks_rejected(), 1);
    }

    #[test]
    fn test_format_hash_rate() {
        assert_eq!(MiningStats::format_hash_rate(500.0), "500.00 H/s");
        assert_eq!(MiningStats::format_hash_rate(1500.0), "1.50 KH/s");
        assert_eq!(MiningStats::format_hash_rate(1_500_000.0), "1.50 MH/s");
        assert_eq!(MiningStats::format_hash_rate(1_500_000_000.0), "1.50 GH/s");
    }

    #[test]
    fn test_uptime_string() {
        let stats = MiningStats::default();
        // Just test it doesn't panic
        let _uptime = stats.uptime_string();
    }
}
