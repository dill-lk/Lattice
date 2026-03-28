//! Receipt types for transaction execution results

use crate::{Address, Amount, BlockHeight, Hash};
use borsh::{BorshDeserialize, BorshSerialize};
use serde::{Deserialize, Serialize};

/// Custom serde support for [u8; 256] bloom filter field.
mod bloom_serde {
    use serde::{Deserializer, Serializer};

    pub fn serialize<S: Serializer>(bloom: &[u8; 256], serializer: S) -> Result<S::Ok, S::Error> {
        serializer.serialize_bytes(bloom)
    }

    pub fn deserialize<'de, D: Deserializer<'de>>(deserializer: D) -> Result<[u8; 256], D::Error> {
        use serde::de::{Error, SeqAccess, Visitor};
        struct BloomVisitor;
        impl<'de> Visitor<'de> for BloomVisitor {
            type Value = [u8; 256];
            fn expecting(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
                write!(f, "a byte array of length 256")
            }
            fn visit_bytes<E: Error>(self, v: &[u8]) -> Result<Self::Value, E> {
                if v.len() != 256 {
                    return Err(E::custom(format!("expected 256 bytes, got {}", v.len())));
                }
                let mut arr = [0u8; 256];
                arr.copy_from_slice(v);
                Ok(arr)
            }
            fn visit_seq<A: SeqAccess<'de>>(self, mut seq: A) -> Result<Self::Value, A::Error> {
                let mut arr = [0u8; 256];
                for (i, byte) in arr.iter_mut().enumerate() {
                    *byte = seq.next_element()?.ok_or_else(|| {
                        A::Error::custom(format!("expected element at index {}", i))
                    })?;
                }
                Ok(arr)
            }
        }
        deserializer.deserialize_bytes(BloomVisitor)
    }
}

/// Transaction receipt containing execution results
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Receipt {
    /// Transaction hash that produced this receipt
    pub transaction_hash: Hash,
    /// Block hash where transaction was included
    pub block_hash: Hash,
    /// Block height
    pub block_height: BlockHeight,
    /// Transaction index in block
    pub transaction_index: u32,
    /// Sender address
    pub from: Address,
    /// Recipient address (or contract address for deployment)
    pub to: Option<Address>,
    /// Contract address (if this was a deployment)
    pub contract_address: Option<Address>,
    /// Gas used by this transaction
    pub gas_used: u64,
    /// Effective gas price paid
    pub effective_gas_price: Amount,
    /// Cumulative gas used in block (up to and including this tx)
    pub cumulative_gas_used: u64,
    /// Transaction status (1 = success, 0 = failed)
    pub status: u8,
    /// Logs/events emitted
    pub logs: Vec<Log>,
    /// Bloom filter for efficient log searching
    #[serde(with = "bloom_serde")]
    pub logs_bloom: [u8; 256],
    /// State root after this transaction
    pub post_state_root: Hash,
    /// Return data from contract call
    pub output: Vec<u8>,
}

/// Parameters for creating a receipt
#[derive(Debug, Clone)]
pub struct ReceiptParams {
    pub transaction_hash: Hash,
    pub block_hash: Hash,
    pub block_height: BlockHeight,
    pub transaction_index: u32,
    pub from: Address,
    pub to: Option<Address>,
    pub gas_used: u64,
    pub effective_gas_price: Amount,
    pub cumulative_gas_used: u64,
    pub post_state_root: Hash,
}

impl Receipt {
    /// Create a successful receipt
    pub fn success(params: ReceiptParams) -> Self {
        Self {
            transaction_hash: params.transaction_hash,
            block_hash: params.block_hash,
            block_height: params.block_height,
            transaction_index: params.transaction_index,
            from: params.from,
            to: params.to,
            contract_address: None,
            gas_used: params.gas_used,
            effective_gas_price: params.effective_gas_price,
            cumulative_gas_used: params.cumulative_gas_used,
            status: 1,
            logs: Vec::new(),
            logs_bloom: [0u8; 256],
            post_state_root: params.post_state_root,
            output: Vec::new(),
        }
    }

    /// Create a failed receipt
    pub fn failed(params: ReceiptParams) -> Self {
        Self {
            transaction_hash: params.transaction_hash,
            block_hash: params.block_hash,
            block_height: params.block_height,
            transaction_index: params.transaction_index,
            from: params.from,
            to: params.to,
            contract_address: None,
            gas_used: params.gas_used,
            effective_gas_price: params.effective_gas_price,
            cumulative_gas_used: params.cumulative_gas_used,
            status: 0,
            logs: Vec::new(),
            logs_bloom: [0u8; 256],
            post_state_root: params.post_state_root,
            output: Vec::new(),
        }
    }

    /// Check if transaction was successful
    pub fn is_success(&self) -> bool {
        self.status == 1
    }

    /// Add a log entry
    pub fn add_log(&mut self, log: Log) {
        // Update bloom filter
        self.update_bloom(&log);
        self.logs.push(log);
    }

    /// Update bloom filter with log data
    fn update_bloom(&mut self, log: &Log) {
        // Add address to bloom
        self.bloom_add(log.address.as_bytes());
        
        // Add topics to bloom
        for topic in &log.topics {
            self.bloom_add(topic);
        }
    }

    /// Add data to bloom filter
    fn bloom_add(&mut self, data: &[u8]) {
        use sha3::{Digest, Sha3_256};
        
        let hash = Sha3_256::digest(data);
        
        // Use 3 bits from hash for bloom filter
        for i in 0..3 {
            let bit_index = ((hash[i * 2] as usize) << 8 | hash[i * 2 + 1] as usize) % 2048;
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;
            
            self.logs_bloom[byte_index] |= 1 << bit_offset;
        }
    }

    /// Check if bloom filter might contain address
    pub fn bloom_contains_address(&self, address: &Address) -> bool {
        self.bloom_contains(address.as_bytes())
    }

    /// Check if bloom filter might contain topic
    pub fn bloom_contains_topic(&self, topic: &[u8; 32]) -> bool {
        self.bloom_contains(topic)
    }

    /// Check if bloom filter might contain data
    fn bloom_contains(&self, data: &[u8]) -> bool {
        use sha3::{Digest, Sha3_256};
        
        let hash = Sha3_256::digest(data);
        
        for i in 0..3 {
            let bit_index = ((hash[i * 2] as usize) << 8 | hash[i * 2 + 1] as usize) % 2048;
            let byte_index = bit_index / 8;
            let bit_offset = bit_index % 8;
            
            if (self.logs_bloom[byte_index] & (1 << bit_offset)) == 0 {
                return false;
            }
        }
        
        true
    }

    /// Get total cost of transaction (gas_used * effective_gas_price)
    pub fn total_cost(&self) -> Amount {
        self.gas_used as Amount * self.effective_gas_price
    }
}

/// Event log emitted by smart contract
#[derive(Debug, Clone, PartialEq, Eq, BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Log {
    /// Address of contract that emitted this log
    pub address: Address,
    /// Indexed topics (up to 4, used for filtering)
    pub topics: Vec<[u8; 32]>,
    /// Arbitrary data
    pub data: Vec<u8>,
    /// Block height where this log was emitted
    pub block_height: BlockHeight,
    /// Transaction hash that produced this log
    pub transaction_hash: Hash,
    /// Transaction index in block
    pub transaction_index: u32,
    /// Log index in transaction
    pub log_index: u32,
    /// Whether this log was removed (chain reorg)
    pub removed: bool,
}

impl Log {
    /// Create a new log
    pub fn new(
        address: Address,
        topics: Vec<[u8; 32]>,
        data: Vec<u8>,
        block_height: BlockHeight,
        transaction_hash: Hash,
        transaction_index: u32,
        log_index: u32,
    ) -> Self {
        Self {
            address,
            topics,
            data,
            block_height,
            transaction_hash,
            transaction_index,
            log_index,
            removed: false,
        }
    }

    /// Check if log matches address filter
    pub fn matches_address(&self, address: &Address) -> bool {
        &self.address == address
    }

    /// Check if log matches topic filter (any of the provided topics)
    pub fn matches_topics(&self, filter_topics: &[[u8; 32]]) -> bool {
        if filter_topics.is_empty() {
            return true;
        }
        
        for filter_topic in filter_topics {
            if self.topics.contains(filter_topic) {
                return true;
            }
        }
        
        false
    }
}

/// Filter for querying logs
#[derive(Debug, Clone, Default)]
pub struct LogFilter {
    /// From block (inclusive)
    pub from_block: Option<BlockHeight>,
    /// To block (inclusive)
    pub to_block: Option<BlockHeight>,
    /// Filter by contract addresses
    pub addresses: Vec<Address>,
    /// Filter by topics (OR logic within each position, AND logic between positions)
    pub topics: Vec<Vec<[u8; 32]>>,
}

impl LogFilter {
    /// Create a new empty filter
    pub fn new() -> Self {
        Self::default()
    }

    /// Set block range
    pub fn with_block_range(mut self, from: BlockHeight, to: BlockHeight) -> Self {
        self.from_block = Some(from);
        self.to_block = Some(to);
        self
    }

    /// Add address filter
    pub fn with_address(mut self, address: Address) -> Self {
        self.addresses.push(address);
        self
    }

    /// Add topic filter
    pub fn with_topic(mut self, topic: [u8; 32]) -> Self {
        if self.topics.is_empty() {
            self.topics.push(vec![topic]);
        } else {
            self.topics[0].push(topic);
        }
        self
    }

    /// Check if a log matches this filter
    pub fn matches(&self, log: &Log) -> bool {
        // Check block range
        if let Some(from) = self.from_block {
            if log.block_height < from {
                return false;
            }
        }
        if let Some(to) = self.to_block {
            if log.block_height > to {
                return false;
            }
        }

        // Check addresses (OR logic)
        if !self.addresses.is_empty() && !self.addresses.contains(&log.address) {
            return false;
        }

        // Check topics (OR within position, AND between positions)
        for (i, topic_options) in self.topics.iter().enumerate() {
            if topic_options.is_empty() {
                continue;
            }

            if i >= log.topics.len() {
                return false;
            }

            if !topic_options.contains(&log.topics[i]) {
                return false;
            }
        }

        true
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_receipt_success() {
        let receipt = Receipt::success(ReceiptParams {
            transaction_hash: [1u8; 32],
            block_hash: [2u8; 32],
            block_height: 100,
            transaction_index: 0,
            from: Address::from_bytes([1u8; 20]),
            to: Some(Address::from_bytes([2u8; 20])),
            gas_used: 21000,
            effective_gas_price: 1,
            cumulative_gas_used: 21000,
            post_state_root: [3u8; 32],
        });

        assert!(receipt.is_success());
        assert_eq!(receipt.gas_used, 21000);
        assert_eq!(receipt.status, 1);
    }

    #[test]
    fn test_receipt_failed() {
        let receipt = Receipt::failed(ReceiptParams {
            transaction_hash: [1u8; 32],
            block_hash: [2u8; 32],
            block_height: 100,
            transaction_index: 0,
            from: Address::from_bytes([1u8; 20]),
            to: Some(Address::from_bytes([2u8; 20])),
            gas_used: 5000,
            effective_gas_price: 1,
            cumulative_gas_used: 5000,
            post_state_root: [3u8; 32],
        });

        assert!(!receipt.is_success());
        assert_eq!(receipt.status, 0);
    }

    #[test]
    fn test_log_creation() {
        let log = Log::new(
            Address::from_bytes([1u8; 20]),
            vec![[1u8; 32], [2u8; 32]],
            vec![1, 2, 3, 4],
            100,
            [5u8; 32],
            0,
            0,
        );

        assert_eq!(log.topics.len(), 2);
        assert_eq!(log.data.len(), 4);
        assert!(!log.removed);
    }

    #[test]
    fn test_bloom_filter() {
        let mut receipt = Receipt::success(ReceiptParams {
            transaction_hash: [1u8; 32],
            block_hash: [2u8; 32],
            block_height: 100,
            transaction_index: 0,
            from: Address::from_bytes([1u8; 20]),
            to: None,
            gas_used: 21000,
            effective_gas_price: 1,
            cumulative_gas_used: 21000,
            post_state_root: [3u8; 32],
        });

        let address = Address::from_bytes([10u8; 20]);
        let log = Log::new(
            address.clone(),
            vec![[5u8; 32]],
            vec![],
            100,
            [1u8; 32],
            0,
            0,
        );

        receipt.add_log(log);

        // Bloom should contain the address
        assert!(receipt.bloom_contains_address(&address));
        
        // Bloom should contain the topic
        assert!(receipt.bloom_contains_topic(&[5u8; 32]));
        
        // Bloom should not contain random data (probably)
        assert!(!receipt.bloom_contains_address(&Address::from_bytes([99u8; 20])));
    }

    #[test]
    fn test_log_filter() {
        let log = Log::new(
            Address::from_bytes([1u8; 20]),
            vec![[1u8; 32], [2u8; 32]],
            vec![],
            100,
            [5u8; 32],
            0,
            0,
        );

        // Filter by address
        let filter = LogFilter::new().with_address(Address::from_bytes([1u8; 20]));
        assert!(filter.matches(&log));

        // Filter by different address
        let filter = LogFilter::new().with_address(Address::from_bytes([2u8; 20]));
        assert!(!filter.matches(&log));

        // Filter by block range
        let filter = LogFilter::new().with_block_range(50, 150);
        assert!(filter.matches(&log));

        let filter = LogFilter::new().with_block_range(150, 200);
        assert!(!filter.matches(&log));
    }
}
