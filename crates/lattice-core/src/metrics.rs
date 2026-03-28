//! Metrics and Telemetry System
//!
//! Prometheus-compatible metrics for monitoring

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::atomic::{AtomicU64, Ordering};
use std::sync::Arc;
use parking_lot::RwLock;

/// Metric types
#[derive(Debug, Clone)]
pub enum MetricType {
    Counter,
    Gauge,
    Histogram,
    Summary,
}

/// Individual metric
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Metric {
    pub name: String,
    pub value: f64,
    pub labels: HashMap<String, String>,
    pub timestamp: u64,
}

/// Counter metric (monotonically increasing)
pub struct Counter {
    value: AtomicU64,
}

impl Counter {
    pub fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }
    
    pub fn inc(&self) {
        self.value.fetch_add(1, Ordering::Relaxed);
    }
    
    pub fn add(&self, amount: u64) {
        self.value.fetch_add(amount, Ordering::Relaxed);
    }
    
    pub fn get(&self) -> u64 {
        self.value.load(Ordering::Relaxed)
    }
}

impl Default for Counter {
    fn default() -> Self {
        Self::new()
    }
}

/// Gauge metric (can go up or down)
pub struct Gauge {
    value: AtomicU64, // Stored as u64, interpreted as f64
}

impl Gauge {
    pub fn new() -> Self {
        Self {
            value: AtomicU64::new(0),
        }
    }
    
    pub fn set(&self, value: f64) {
        self.value.store(value.to_bits(), Ordering::Relaxed);
    }
    
    pub fn inc(&self) {
        let current = f64::from_bits(self.value.load(Ordering::Relaxed));
        self.set(current + 1.0);
    }
    
    pub fn dec(&self) {
        let current = f64::from_bits(self.value.load(Ordering::Relaxed));
        self.set(current - 1.0);
    }
    
    pub fn add(&self, amount: f64) {
        let current = f64::from_bits(self.value.load(Ordering::Relaxed));
        self.set(current + amount);
    }
    
    pub fn get(&self) -> f64 {
        f64::from_bits(self.value.load(Ordering::Relaxed))
    }
}

impl Default for Gauge {
    fn default() -> Self {
        Self::new()
    }
}

/// Histogram for tracking distributions
pub struct Histogram {
    buckets: Vec<(f64, AtomicU64)>, // (upper_bound, count)
    sum: AtomicU64,
    count: AtomicU64,
}

impl Histogram {
    pub fn new(buckets: Vec<f64>) -> Self {
        let bucket_vec = buckets
            .into_iter()
            .map(|bound| (bound, AtomicU64::new(0)))
            .collect();
        
        Self {
            buckets: bucket_vec,
            sum: AtomicU64::new(0),
            count: AtomicU64::new(0),
        }
    }
    
    pub fn observe(&self, value: f64) {
        // Update sum (store as bits)
        let current_sum = f64::from_bits(self.sum.load(Ordering::Relaxed));
        self.sum.store((current_sum + value).to_bits(), Ordering::Relaxed);
        
        // Update count
        self.count.fetch_add(1, Ordering::Relaxed);
        
        // Update buckets
        for (bound, counter) in &self.buckets {
            if value <= *bound {
                counter.fetch_add(1, Ordering::Relaxed);
            }
        }
    }
    
    pub fn get_count(&self) -> u64 {
        self.count.load(Ordering::Relaxed)
    }
    
    pub fn get_sum(&self) -> f64 {
        f64::from_bits(self.sum.load(Ordering::Relaxed))
    }
}

/// Metrics registry
pub struct MetricsRegistry {
    counters: Arc<RwLock<HashMap<String, Arc<Counter>>>>,
    gauges: Arc<RwLock<HashMap<String, Arc<Gauge>>>>,
    histograms: Arc<RwLock<HashMap<String, Arc<Histogram>>>>,
}

impl MetricsRegistry {
    pub fn new() -> Self {
        Self {
            counters: Arc::new(RwLock::new(HashMap::new())),
            gauges: Arc::new(RwLock::new(HashMap::new())),
            histograms: Arc::new(RwLock::new(HashMap::new())),
        }
    }
    
    /// Register or get counter
    pub fn counter(&self, name: &str) -> Arc<Counter> {
        let mut counters = self.counters.write();
        counters
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Counter::new()))
            .clone()
    }
    
    /// Register or get gauge
    pub fn gauge(&self, name: &str) -> Arc<Gauge> {
        let mut gauges = self.gauges.write();
        gauges
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Gauge::new()))
            .clone()
    }
    
    /// Register or get histogram
    pub fn histogram(&self, name: &str, buckets: Vec<f64>) -> Arc<Histogram> {
        let mut histograms = self.histograms.write();
        histograms
            .entry(name.to_string())
            .or_insert_with(|| Arc::new(Histogram::new(buckets)))
            .clone()
    }
    
    /// Export metrics in Prometheus format
    pub fn export_prometheus(&self) -> String {
        let mut output = String::new();
        
        // Export counters
        let counters = self.counters.read();
        for (name, counter) in counters.iter() {
            output.push_str(&format!("# TYPE {} counter\n", name));
            output.push_str(&format!("{} {}\n", name, counter.get()));
        }
        
        // Export gauges
        let gauges = self.gauges.read();
        for (name, gauge) in gauges.iter() {
            output.push_str(&format!("# TYPE {} gauge\n", name));
            output.push_str(&format!("{} {}\n", name, gauge.get()));
        }
        
        // Export histograms
        let histograms = self.histograms.read();
        for (name, histogram) in histograms.iter() {
            output.push_str(&format!("# TYPE {} histogram\n", name));
            output.push_str(&format!("{}_count {}\n", name, histogram.get_count()));
            output.push_str(&format!("{}_sum {}\n", name, histogram.get_sum()));
        }
        
        output
    }
    
    /// Export metrics as JSON
    pub fn export_json(&self) -> serde_json::Value {
        let mut metrics = serde_json::Map::new();
        
        // Export counters
        let counters = self.counters.read();
        let mut counter_map = serde_json::Map::new();
        for (name, counter) in counters.iter() {
            counter_map.insert(name.clone(), serde_json::json!(counter.get()));
        }
        metrics.insert("counters".to_string(), serde_json::Value::Object(counter_map));
        
        // Export gauges
        let gauges = self.gauges.read();
        let mut gauge_map = serde_json::Map::new();
        for (name, gauge) in gauges.iter() {
            gauge_map.insert(name.clone(), serde_json::json!(gauge.get()));
        }
        metrics.insert("gauges".to_string(), serde_json::Value::Object(gauge_map));
        
        serde_json::Value::Object(metrics)
    }
}

impl Default for MetricsRegistry {
    fn default() -> Self {
        Self::new()
    }
}

/// Blockchain-specific metrics
pub struct BlockchainMetrics {
    pub registry: MetricsRegistry,
    
    // Block metrics
    pub blocks_processed: Arc<Counter>,
    pub blocks_validated: Arc<Counter>,
    pub blocks_rejected: Arc<Counter>,
    pub current_height: Arc<Gauge>,
    pub block_processing_time: Arc<Histogram>,
    
    // Transaction metrics
    pub transactions_processed: Arc<Counter>,
    pub transactions_validated: Arc<Counter>,
    pub transactions_rejected: Arc<Counter>,
    pub mempool_size: Arc<Gauge>,
    pub tx_processing_time: Arc<Histogram>,
    
    // Network metrics
    pub peers_connected: Arc<Gauge>,
    pub messages_sent: Arc<Counter>,
    pub messages_received: Arc<Counter>,
    pub bytes_sent: Arc<Counter>,
    pub bytes_received: Arc<Counter>,
    
    // Consensus metrics
    pub mining_attempts: Arc<Counter>,
    pub blocks_mined: Arc<Counter>,
    pub current_difficulty: Arc<Gauge>,
    
    // Storage metrics
    pub db_reads: Arc<Counter>,
    pub db_writes: Arc<Counter>,
    pub db_size: Arc<Gauge>,
    
    // Performance metrics
    pub cpu_usage: Arc<Gauge>,
    pub memory_usage: Arc<Gauge>,
}

impl BlockchainMetrics {
    pub fn new() -> Self {
        let registry = MetricsRegistry::new();
        
        Self {
            blocks_processed: registry.counter("blocks_processed_total"),
            blocks_validated: registry.counter("blocks_validated_total"),
            blocks_rejected: registry.counter("blocks_rejected_total"),
            current_height: registry.gauge("blockchain_height"),
            block_processing_time: registry.histogram(
                "block_processing_seconds",
                vec![0.001, 0.01, 0.1, 0.5, 1.0, 5.0, 10.0],
            ),
            
            transactions_processed: registry.counter("transactions_processed_total"),
            transactions_validated: registry.counter("transactions_validated_total"),
            transactions_rejected: registry.counter("transactions_rejected_total"),
            mempool_size: registry.gauge("mempool_transactions"),
            tx_processing_time: registry.histogram(
                "transaction_processing_seconds",
                vec![0.0001, 0.001, 0.01, 0.1, 1.0],
            ),
            
            peers_connected: registry.gauge("network_peers_connected"),
            messages_sent: registry.counter("network_messages_sent_total"),
            messages_received: registry.counter("network_messages_received_total"),
            bytes_sent: registry.counter("network_bytes_sent_total"),
            bytes_received: registry.counter("network_bytes_received_total"),
            
            mining_attempts: registry.counter("mining_attempts_total"),
            blocks_mined: registry.counter("blocks_mined_total"),
            current_difficulty: registry.gauge("consensus_difficulty"),
            
            db_reads: registry.counter("database_reads_total"),
            db_writes: registry.counter("database_writes_total"),
            db_size: registry.gauge("database_size_bytes"),
            
            cpu_usage: registry.gauge("system_cpu_usage_percent"),
            memory_usage: registry.gauge("system_memory_usage_bytes"),
            
            registry,
        }
    }
    
    /// Export all metrics
    pub fn export_prometheus(&self) -> String {
        self.registry.export_prometheus()
    }
    
    /// Export all metrics as JSON
    pub fn export_json(&self) -> serde_json::Value {
        self.registry.export_json()
    }
}

impl Default for BlockchainMetrics {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_counter() {
        let counter = Counter::new();
        counter.inc();
        counter.add(5);
        assert_eq!(counter.get(), 6);
    }
    
    #[test]
    fn test_gauge() {
        let gauge = Gauge::new();
        gauge.set(10.5);
        gauge.inc();
        assert_eq!(gauge.get(), 11.5);
        gauge.dec();
        assert_eq!(gauge.get(), 10.5);
    }
    
    #[test]
    fn test_histogram() {
        let histogram = Histogram::new(vec![1.0, 5.0, 10.0]);
        histogram.observe(0.5);
        histogram.observe(2.0);
        histogram.observe(7.0);
        
        assert_eq!(histogram.get_count(), 3);
        assert_eq!(histogram.get_sum(), 9.5);
    }
    
    #[test]
    fn test_registry() {
        let registry = MetricsRegistry::new();
        
        let counter = registry.counter("test_counter");
        counter.inc();
        counter.inc();
        
        let output = registry.export_prometheus();
        assert!(output.contains("test_counter 2"));
    }
    
    #[test]
    fn test_blockchain_metrics() {
        let metrics = BlockchainMetrics::new();
        
        metrics.blocks_processed.inc();
        metrics.current_height.set(100.0);
        metrics.block_processing_time.observe(0.5);
        
        let json = metrics.export_json();
        assert!(json.is_object());
    }
}
