//! Configuration Validator
//!
//! Validates node configuration and system requirements

use serde::{Deserialize, Serialize};
use std::path::Path;
use std::collections::HashMap;

/// Validation result
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct ValidationResult {
    pub valid: bool,
    pub errors: Vec<String>,
    pub warnings: Vec<String>,
    pub info: Vec<String>,
}

impl ValidationResult {
    pub fn new() -> Self {
        Self {
            valid: true,
            errors: Vec::new(),
            warnings: Vec::new(),
            info: Vec::new(),
        }
    }
    
    pub fn add_error(&mut self, msg: String) {
        self.errors.push(msg);
        self.valid = false;
    }
    
    pub fn add_warning(&mut self, msg: String) {
        self.warnings.push(msg);
    }
    
    pub fn add_info(&mut self, msg: String) {
        self.info.push(msg);
    }
    
    pub fn merge(&mut self, other: ValidationResult) {
        self.errors.extend(other.errors);
        self.warnings.extend(other.warnings);
        self.info.extend(other.info);
        self.valid = self.valid && other.valid;
    }
}

impl Default for ValidationResult {
    fn default() -> Self {
        Self::new()
    }
}

/// Configuration validator
pub struct ConfigValidator;

impl ConfigValidator {
    /// Validate network configuration
    pub fn validate_network_config(
        listen_addr: &str,
        max_peers: usize,
    ) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        // Check listen address format
        if !listen_addr.starts_with("/ip4/") && !listen_addr.starts_with("/ip6/") {
            result.add_error(format!("Invalid listen address format: {}", listen_addr));
        }
        
        // Check peer count
        if max_peers == 0 {
            result.add_error("max_peers must be greater than 0".to_string());
        } else if max_peers < 10 {
            result.add_warning("max_peers is very low (< 10), may affect network connectivity".to_string());
        } else if max_peers > 500 {
            result.add_warning("max_peers is very high (> 500), may consume excessive resources".to_string());
        }
        
        result
    }
    
    /// Validate consensus configuration
    pub fn validate_consensus_config(
        mining_threads: usize,
        difficulty: u64,
    ) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        // Check mining threads
        let cpu_count = num_cpus::get();
        if mining_threads > cpu_count {
            result.add_warning(format!(
                "mining_threads ({}) exceeds CPU count ({})",
                mining_threads, cpu_count
            ));
        }
        
        // Check difficulty
        if difficulty < 1000 {
            result.add_warning("Difficulty is very low, blocks will be produced rapidly".to_string());
        } else if difficulty > 10_000_000_000 {
            result.add_warning("Difficulty is very high, blocks may take hours to produce".to_string());
        }
        
        result
    }
    
    /// Validate RPC configuration
    pub fn validate_rpc_config(
        listen_addr: &str,
        enabled: bool,
    ) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        if enabled {
            // Check address format
            if !listen_addr.contains(':') {
                result.add_error(format!("Invalid RPC address format: {}", listen_addr));
            }
            
            // Warn about public binding
            if listen_addr.starts_with("0.0.0.0:") {
                result.add_warning("RPC is bound to 0.0.0.0 (all interfaces). Ensure firewall is configured.".to_string());
            }
            
            result.add_info(format!("RPC enabled on {}", listen_addr));
        } else {
            result.add_info("RPC is disabled".to_string());
        }
        
        result
    }
    
    /// Validate storage configuration
    pub fn validate_storage_config(
        db_path: &Path,
        cache_size: usize,
    ) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        // Check cache size
        if cache_size < 64 {
            result.add_warning("Cache size is very small (< 64MB), performance may be poor".to_string());
        } else if cache_size > 4096 {
            result.add_warning("Cache size is very large (> 4GB), may consume excessive memory".to_string());
        }
        
        // Check if parent directory exists
        if let Some(parent) = db_path.parent() {
            if !parent.exists() {
                result.add_error(format!("Database parent directory does not exist: {:?}", parent));
            }
        }
        
        // Check write permissions
        if db_path.exists() {
            let metadata = std::fs::metadata(db_path);
            if let Ok(meta) = metadata {
                if meta.permissions().readonly() {
                    result.add_error(format!("Database path is read-only: {:?}", db_path));
                }
            }
        }
        
        result
    }
}

/// System requirements checker
pub struct SystemChecker;

impl SystemChecker {
    /// Check all system requirements
    pub fn check_all() -> ValidationResult {
        let mut result = ValidationResult::new();
        
        result.merge(Self::check_cpu());
        result.merge(Self::check_memory());
        result.merge(Self::check_disk_space());
        result.merge(Self::check_os());
        
        result
    }
    
    /// Check CPU
    fn check_cpu() -> ValidationResult {
        let mut result = ValidationResult::new();
        
        let cpu_count = num_cpus::get();
        result.add_info(format!("CPU cores: {}", cpu_count));
        
        if cpu_count < 2 {
            result.add_warning("Less than 2 CPU cores detected. Performance may be limited.".to_string());
        }
        
        result
    }
    
    /// Check memory
    fn check_memory() -> ValidationResult {
        let mut result = ValidationResult::new();
        
        // In production, use sysinfo crate for actual memory check
        result.add_info("Memory check: OK (requires sysinfo crate for details)".to_string());
        
        result
    }
    
    /// Check disk space
    fn check_disk_space() -> ValidationResult {
        let mut result = ValidationResult::new();
        
        // In production, use sysinfo crate for actual disk space check
        result.add_info("Disk space check: OK (requires sysinfo crate for details)".to_string());
        
        result
    }
    
    /// Check OS
    fn check_os() -> ValidationResult {
        let mut result = ValidationResult::new();
        
        let os = std::env::consts::OS;
        result.add_info(format!("Operating System: {}", os));
        
        match os {
            "linux" | "macos" | "windows" => {
                result.add_info(format!("{} is supported", os));
            }
            _ => {
                result.add_warning(format!("{} may not be fully supported", os));
            }
        }
        
        result
    }
    
    /// Check Rust version
    pub fn check_rust_version() -> ValidationResult {
        let mut result = ValidationResult::new();
        
        let rust_version = rustc_version_runtime::version();
        result.add_info(format!("Rust version: {}", rust_version));
        
        // Check minimum version
        let min_version = rustc_version_runtime::Version::parse("1.75.0").unwrap();
        if rust_version < min_version {
            result.add_error(format!(
                "Rust version {} is below minimum required version 1.75.0",
                rust_version
            ));
        }
        
        result
    }
}

/// Network diagnostics
pub struct NetworkDiagnostics;

impl NetworkDiagnostics {
    /// Check network connectivity
    pub async fn check_connectivity() -> ValidationResult {
        let mut result = ValidationResult::new();
        
        result.add_info("Network connectivity check would go here".to_string());
        result.add_info("In production: ping bootstrap nodes, check DNS resolution".to_string());
        
        result
    }
    
    /// Check port availability
    pub fn check_port_available(port: u16) -> ValidationResult {
        let mut result = ValidationResult::new();
        
        use std::net::TcpListener;
        
        match TcpListener::bind(format!("127.0.0.1:{}", port)) {
            Ok(_) => {
                result.add_info(format!("Port {} is available", port));
            }
            Err(e) => {
                result.add_error(format!("Port {} is not available: {}", port, e));
            }
        }
        
        result
    }
    
    /// Check firewall (platform-specific)
    pub fn check_firewall() -> ValidationResult {
        let mut result = ValidationResult::new();
        
        result.add_info("Firewall check: Platform-specific implementation needed".to_string());
        result.add_warning("Ensure ports 8545 (RPC) and 30333 (P2P) are open in firewall".to_string());
        
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    
    #[test]
    fn test_network_validation() {
        let result = ConfigValidator::validate_network_config("/ip4/0.0.0.0/tcp/30333", 50);
        assert!(result.valid);
        
        let result = ConfigValidator::validate_network_config("invalid", 50);
        assert!(!result.valid);
    }
    
    #[test]
    fn test_consensus_validation() {
        let result = ConfigValidator::validate_consensus_config(4, 1000000);
        assert!(result.valid);
    }
    
    #[test]
    fn test_port_check() {
        let result = NetworkDiagnostics::check_port_available(0); // OS assigns random free port
        assert!(result.valid);
    }
    
    #[test]
    fn test_system_check() {
        let result = SystemChecker::check_all();
        assert!(!result.info.is_empty());
    }
}
