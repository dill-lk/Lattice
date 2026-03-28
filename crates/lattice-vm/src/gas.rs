//! Gas metering for contract execution
//!
//! Gas is the unit of computation cost in the VM. Each operation
//! consumes a specific amount of gas, and execution fails if gas
//! is exhausted.

use crate::error::{Result, VmError};

/// Gas costs for various operations
#[derive(Debug, Clone, Copy)]
pub struct GasCosts {
    /// Base cost for any instruction
    pub base: u64,
    /// Cost per byte of memory allocation
    pub memory_byte: u64,
    /// Cost for storage read
    pub storage_read: u64,
    /// Cost for storage write (new key)
    pub storage_write_new: u64,
    /// Cost for storage write (existing key)
    pub storage_write_existing: u64,
    /// Cost for storage delete
    pub storage_delete: u64,
    /// Cost per byte of storage value
    pub storage_byte: u64,
    /// Cost for SHA3 hash (base)
    pub sha3_base: u64,
    /// Cost per word (32 bytes) for SHA3
    pub sha3_word: u64,
    /// Cost for signature verification
    pub signature_verify: u64,
    /// Cost for external call (base)
    pub call_base: u64,
    /// Cost per byte of call data
    pub call_data_byte: u64,
    /// Cost for emitting a log
    pub log_base: u64,
    /// Cost per byte of log data
    pub log_byte: u64,
    /// Cost per log topic
    pub log_topic: u64,
    /// Cost for getting block info
    pub block_info: u64,
    /// Cost for getting caller address
    pub caller: u64,
    /// Cost for getting contract address
    pub address: u64,
    /// Cost for getting balance
    pub balance: u64,
    /// Cost for contract creation (base)
    pub create_base: u64,
    /// Cost per byte of contract code
    pub create_byte: u64,
}

impl Default for GasCosts {
    fn default() -> Self {
        Self {
            base: 1,
            memory_byte: 3,
            storage_read: 200,
            storage_write_new: 20000,
            storage_write_existing: 5000,
            storage_delete: 5000,
            storage_byte: 50,
            sha3_base: 30,
            sha3_word: 6,
            signature_verify: 3000,
            call_base: 700,
            call_data_byte: 3,
            log_base: 375,
            log_byte: 8,
            log_topic: 375,
            block_info: 2,
            caller: 2,
            address: 2,
            balance: 400,
            create_base: 32000,
            create_byte: 200,
        }
    }
}

/// Gas meter for tracking and limiting gas consumption
#[derive(Debug, Clone)]
pub struct GasMeter {
    /// Maximum gas allowed
    limit: u64,
    /// Gas consumed so far
    used: u64,
    /// Gas costs configuration
    costs: GasCosts,
}

impl GasMeter {
    /// Create a new gas meter with the given limit
    pub fn new(limit: u64) -> Self {
        Self {
            limit,
            used: 0,
            costs: GasCosts::default(),
        }
    }

    /// Create a new gas meter with custom costs
    pub fn with_costs(limit: u64, costs: GasCosts) -> Self {
        Self {
            limit,
            used: 0,
            costs,
        }
    }

    /// Get the gas limit
    pub fn limit(&self) -> u64 {
        self.limit
    }

    /// Get the gas used so far
    pub fn used(&self) -> u64 {
        self.used
    }

    /// Get the remaining gas
    pub fn remaining(&self) -> u64 {
        self.limit.saturating_sub(self.used)
    }

    /// Get the gas costs configuration
    pub fn costs(&self) -> &GasCosts {
        &self.costs
    }

    /// Charge gas for an operation, returns error if insufficient
    pub fn charge(&mut self, amount: u64) -> Result<()> {
        let new_used = self.used.saturating_add(amount);
        if new_used > self.limit {
            return Err(VmError::OutOfGas {
                required: amount,
                available: self.remaining(),
            });
        }
        self.used = new_used;
        Ok(())
    }

    /// Charge gas for storage read
    pub fn charge_storage_read(&mut self) -> Result<()> {
        self.charge(self.costs.storage_read)
    }

    /// Charge gas for storage write
    pub fn charge_storage_write(&mut self, is_new: bool, value_len: usize) -> Result<()> {
        let base_cost = if is_new {
            self.costs.storage_write_new
        } else {
            self.costs.storage_write_existing
        };
        let byte_cost = (value_len as u64).saturating_mul(self.costs.storage_byte);
        self.charge(base_cost.saturating_add(byte_cost))
    }

    /// Charge gas for storage delete
    pub fn charge_storage_delete(&mut self) -> Result<()> {
        self.charge(self.costs.storage_delete)
    }

    /// Charge gas for SHA3 hashing
    pub fn charge_sha3(&mut self, data_len: usize) -> Result<()> {
        let words = (data_len as u64 + 31) / 32;
        let cost = self.costs.sha3_base + words * self.costs.sha3_word;
        self.charge(cost)
    }

    /// Charge gas for signature verification
    pub fn charge_signature_verify(&mut self) -> Result<()> {
        self.charge(self.costs.signature_verify)
    }

    /// Charge gas for memory allocation
    pub fn charge_memory(&mut self, bytes: usize) -> Result<()> {
        let cost = (bytes as u64).saturating_mul(self.costs.memory_byte);
        self.charge(cost)
    }

    /// Charge gas for external call
    pub fn charge_call(&mut self, data_len: usize) -> Result<()> {
        let cost = self.costs.call_base + (data_len as u64) * self.costs.call_data_byte;
        self.charge(cost)
    }

    /// Charge gas for log emission
    pub fn charge_log(&mut self, data_len: usize, num_topics: usize) -> Result<()> {
        let cost = self.costs.log_base
            + (data_len as u64) * self.costs.log_byte
            + (num_topics as u64) * self.costs.log_topic;
        self.charge(cost)
    }

    /// Charge gas for contract creation
    pub fn charge_create(&mut self, code_len: usize) -> Result<()> {
        let cost = self.costs.create_base + (code_len as u64) * self.costs.create_byte;
        self.charge(cost)
    }

    /// Refund gas (e.g., for storage deletion)
    pub fn refund(&mut self, amount: u64) {
        self.used = self.used.saturating_sub(amount);
    }

    /// Create a sub-meter for nested execution
    pub fn sub_meter(&self, gas_limit: u64) -> Result<Self> {
        let available = self.remaining();
        if gas_limit > available {
            return Err(VmError::GasLimitExceeded {
                limit: available,
                requested: gas_limit,
            });
        }
        Ok(Self {
            limit: gas_limit,
            used: 0,
            costs: self.costs.clone(),
        })
    }

    /// Consume gas from sub-meter after nested execution
    pub fn consume_sub_meter(&mut self, sub_meter: &GasMeter) -> Result<()> {
        self.charge(sub_meter.used())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_gas_meter_basic() {
        let mut meter = GasMeter::new(1000);
        assert_eq!(meter.limit(), 1000);
        assert_eq!(meter.used(), 0);
        assert_eq!(meter.remaining(), 1000);

        meter.charge(100).unwrap();
        assert_eq!(meter.used(), 100);
        assert_eq!(meter.remaining(), 900);
    }

    #[test]
    fn test_gas_exhaustion() {
        let mut meter = GasMeter::new(100);
        meter.charge(50).unwrap();

        let result = meter.charge(100);
        assert!(matches!(result, Err(VmError::OutOfGas { .. })));
    }

    #[test]
    fn test_gas_refund() {
        let mut meter = GasMeter::new(1000);
        meter.charge(500).unwrap();
        meter.refund(200);
        assert_eq!(meter.used(), 300);
    }

    #[test]
    fn test_sub_meter() {
        let meter = GasMeter::new(1000);
        let sub = meter.sub_meter(500).unwrap();
        assert_eq!(sub.limit(), 500);
        assert_eq!(sub.used(), 0);
    }

    #[test]
    fn test_sub_meter_exceeds_parent() {
        let mut meter = GasMeter::new(1000);
        meter.charge(800).unwrap();

        let result = meter.sub_meter(500);
        assert!(matches!(result, Err(VmError::GasLimitExceeded { .. })));
    }

    #[test]
    fn test_storage_write_cost() {
        let mut meter = GasMeter::new(100000);

        // New key costs more
        meter.charge_storage_write(true, 32).unwrap();
        let cost_new = meter.used();

        let mut meter2 = GasMeter::new(100000);
        meter2.charge_storage_write(false, 32).unwrap();
        let cost_existing = meter2.used();

        assert!(cost_new > cost_existing);
    }
}
