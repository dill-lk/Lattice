//! Advanced VM Opcodes for WASM smart contracts
//! 
//! Extended instruction set including:
//! - Advanced mathematics (exponentials, logarithms, roots)
//! - Cryptographic operations (hashing, signatures, VRF)
//! - Data structures (trees, graphs)

use lattice_crypto::{sha3_256, Keypair, PublicKey};
use std::collections::HashMap;
use thiserror::Error;

/// VM Opcode errors
#[derive(Debug, Error)]
pub enum OpcodeError {
    #[error("division by zero")]
    DivisionByZero,
    
    #[error("overflow in arithmetic operation")]
    Overflow,
    
    #[error("underflow in arithmetic operation")]
    Underflow,
    
    #[error("invalid input: {0}")]
    InvalidInput(String),
    
    #[error("cryptographic operation failed: {0}")]
    CryptoError(String),
    
    #[error("stack underflow")]
    StackUnderflow,
    
    #[error("invalid opcode: {0}")]
    InvalidOpcode(u8),
}

pub type Result<T> = std::result::Result<T, OpcodeError>;

/// Advanced VM opcodes
#[repr(u8)]
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum AdvancedOpcode {
    // Mathematical operations (0x80-0x9F)
    Exp = 0x80,           // a^b
    Log = 0x81,           // log_b(a)
    Sqrt = 0x82,          // √a
    Cbrt = 0x83,          // ∛a
    Pow10 = 0x84,         // 10^a
    Factorial = 0x85,     // a!
    Gcd = 0x86,           // gcd(a, b)
    Lcm = 0x87,           // lcm(a, b)
    ModExp = 0x88,        // (base^exp) mod modulus
    ModInv = 0x89,        // modular inverse
    
    // Bitwise operations (0x90-0x9F)
    RotateLeft = 0x90,    // rotate left
    RotateRight = 0x91,   // rotate right
    PopCount = 0x92,      // count set bits
    Clz = 0x93,           // count leading zeros
    Ctz = 0x94,           // count trailing zeros
    
    // Cryptographic operations (0xA0-0xBF)
    Sha3_256 = 0xA0,      // SHA3-256 hash
    Sha3_512 = 0xA1,      // SHA3-512 hash
    Blake3 = 0xA2,        // BLAKE3 hash
    Keccak256 = 0xA3,     // Keccak-256 hash
    
    DilithiumVerify = 0xA4,  // Verify Dilithium signature
    DilithiumSign = 0xA5,    // Sign with Dilithium (restricted)
    
    KyberEncrypt = 0xA6,     // Kyber key encapsulation
    KyberDecrypt = 0xA7,     // Kyber decapsulation
    
    HashToPoint = 0xA8,      // Hash to elliptic curve point
    VrfVerify = 0xA9,        // Verify VRF proof
    VrfProve = 0xAA,         // Generate VRF proof (restricted)
    
    // Data structure operations (0xC0-0xDF)
    MerkleRoot = 0xC0,       // Compute merkle root
    MerkleVerify = 0xC1,     // Verify merkle proof
    MmrRoot = 0xC2,          // Compute MMR root
    MmrAppend = 0xC3,        // Append to MMR
    MmrVerify = 0xC4,        // Verify MMR proof
    
    BloomAdd = 0xC5,         // Add to bloom filter
    BloomContains = 0xC6,    // Check bloom filter
    
    // State operations (0xE0-0xFF)
    StateGet = 0xE0,         // Get from state
    StateSet = 0xE1,         // Set in state
    StateDelete = 0xE2,      // Delete from state
    StateExists = 0xE3,      // Check existence
    
    AccountBalance = 0xE4,   // Get account balance
    AccountNonce = 0xE5,     // Get account nonce
    
    Emit = 0xF0,             // Emit event log
    Call = 0xF1,             // Call another contract
    DelegateCall = 0xF2,     // Delegate call
    StaticCall = 0xF3,       // Static call (read-only)
    
    Revert = 0xFE,           // Revert execution
    Return = 0xFF,           // Return from execution
}

impl TryFrom<u8> for AdvancedOpcode {
    type Error = OpcodeError;
    
    fn try_from(value: u8) -> Result<Self> {
        match value {
            0x80 => Ok(Self::Exp),
            0x81 => Ok(Self::Log),
            0x82 => Ok(Self::Sqrt),
            0x83 => Ok(Self::Cbrt),
            0x84 => Ok(Self::Pow10),
            0x85 => Ok(Self::Factorial),
            0x86 => Ok(Self::Gcd),
            0x87 => Ok(Self::Lcm),
            0x88 => Ok(Self::ModExp),
            0x89 => Ok(Self::ModInv),
            
            0x90 => Ok(Self::RotateLeft),
            0x91 => Ok(Self::RotateRight),
            0x92 => Ok(Self::PopCount),
            0x93 => Ok(Self::Clz),
            0x94 => Ok(Self::Ctz),
            
            0xA0 => Ok(Self::Sha3_256),
            0xA1 => Ok(Self::Sha3_512),
            0xA2 => Ok(Self::Blake3),
            0xA3 => Ok(Self::Keccak256),
            0xA4 => Ok(Self::DilithiumVerify),
            0xA5 => Ok(Self::DilithiumSign),
            0xA6 => Ok(Self::KyberEncrypt),
            0xA7 => Ok(Self::KyberDecrypt),
            0xA8 => Ok(Self::HashToPoint),
            0xA9 => Ok(Self::VrfVerify),
            0xAA => Ok(Self::VrfProve),
            
            0xC0 => Ok(Self::MerkleRoot),
            0xC1 => Ok(Self::MerkleVerify),
            0xC2 => Ok(Self::MmrRoot),
            0xC3 => Ok(Self::MmrAppend),
            0xC4 => Ok(Self::MmrVerify),
            0xC5 => Ok(Self::BloomAdd),
            0xC6 => Ok(Self::BloomContains),
            
            0xE0 => Ok(Self::StateGet),
            0xE1 => Ok(Self::StateSet),
            0xE2 => Ok(Self::StateDelete),
            0xE3 => Ok(Self::StateExists),
            0xE4 => Ok(Self::AccountBalance),
            0xE5 => Ok(Self::AccountNonce),
            
            0xF0 => Ok(Self::Emit),
            0xF1 => Ok(Self::Call),
            0xF2 => Ok(Self::DelegateCall),
            0xF3 => Ok(Self::StaticCall),
            0xFE => Ok(Self::Revert),
            0xFF => Ok(Self::Return),
            
            _ => Err(OpcodeError::InvalidOpcode(value)),
        }
    }
}

/// VM execution stack
#[derive(Debug, Clone)]
pub struct VmStack {
    stack: Vec<u64>,
    max_depth: usize,
}

impl VmStack {
    pub fn new(max_depth: usize) -> Self {
        Self {
            stack: Vec::new(),
            max_depth,
        }
    }
    
    pub fn push(&mut self, value: u64) -> Result<()> {
        if self.stack.len() >= self.max_depth {
            return Err(OpcodeError::Overflow);
        }
        self.stack.push(value);
        Ok(())
    }
    
    pub fn pop(&mut self) -> Result<u64> {
        self.stack.pop().ok_or(OpcodeError::StackUnderflow)
    }
    
    pub fn peek(&self) -> Result<u64> {
        self.stack.last().copied().ok_or(OpcodeError::StackUnderflow)
    }
    
    pub fn depth(&self) -> usize {
        self.stack.len()
    }
}

/// Advanced mathematical operations
pub mod math_ops {
    use super::*;
    
    /// Compute a^b with overflow checking
    pub fn exp(base: u64, exp: u64) -> Result<u64> {
        if exp == 0 {
            return Ok(1);
        }
        base.checked_pow(exp as u32).ok_or(OpcodeError::Overflow)
    }
    
    /// Compute log_b(a)
    pub fn log(value: u64, base: u64) -> Result<u64> {
        if value == 0 || base <= 1 {
            return Err(OpcodeError::InvalidInput("invalid log parameters".to_string()));
        }
        Ok((value as f64).log(base as f64) as u64)
    }
    
    /// Compute √a
    pub fn sqrt(value: u64) -> Result<u64> {
        Ok((value as f64).sqrt() as u64)
    }
    
    /// Compute ∛a
    pub fn cbrt(value: u64) -> Result<u64> {
        Ok((value as f64).cbrt() as u64)
    }
    
    /// Compute a! (factorial)
    pub fn factorial(n: u64) -> Result<u64> {
        if n > 20 {
            return Err(OpcodeError::Overflow);
        }
        Ok((1..=n).product())
    }
    
    /// Greatest common divisor
    pub fn gcd(mut a: u64, mut b: u64) -> u64 {
        while b != 0 {
            let temp = b;
            b = a % b;
            a = temp;
        }
        a
    }
    
    /// Least common multiple
    pub fn lcm(a: u64, b: u64) -> Result<u64> {
        if a == 0 || b == 0 {
            return Ok(0);
        }
        a.checked_mul(b / gcd(a, b)).ok_or(OpcodeError::Overflow)
    }
    
    /// Modular exponentiation: (base^exp) mod modulus
    pub fn mod_exp(base: u64, exp: u64, modulus: u64) -> Result<u64> {
        if modulus == 0 {
            return Err(OpcodeError::DivisionByZero);
        }
        
        let mut result = 1u64;
        let mut base = base % modulus;
        let mut exp = exp;
        
        while exp > 0 {
            if exp % 2 == 1 {
                result = (result as u128 * base as u128 % modulus as u128) as u64;
            }
            exp >>= 1;
            base = (base as u128 * base as u128 % modulus as u128) as u64;
        }
        
        Ok(result)
    }
    
    /// Modular inverse using extended Euclidean algorithm
    pub fn mod_inv(a: u64, modulus: u64) -> Result<u64> {
        if modulus == 0 {
            return Err(OpcodeError::DivisionByZero);
        }
        
        let (mut t, mut newt) = (0i128, 1i128);
        let (mut r, mut newr) = (modulus as i128, a as i128);
        
        while newr != 0 {
            let quotient = r / newr;
            (t, newt) = (newt, t - quotient * newt);
            (r, newr) = (newr, r - quotient * newr);
        }
        
        if r > 1 {
            return Err(OpcodeError::InvalidInput("not invertible".to_string()));
        }
        if t < 0 {
            t += modulus as i128;
        }
        
        Ok(t as u64)
    }
}

/// Cryptographic operations
pub mod crypto_ops {
    use super::*;
    use sha3::{Digest, Sha3_256, Sha3_512};
    
    /// SHA3-256 hash
    pub fn sha3_256_op(data: &[u8]) -> [u8; 32] {
        let hash = Sha3_256::digest(data);
        let mut result = [0u8; 32];
        result.copy_from_slice(&hash);
        result
    }
    
    /// SHA3-512 hash
    pub fn sha3_512_op(data: &[u8]) -> [u8; 64] {
        let hash = Sha3_512::digest(data);
        let mut result = [0u8; 64];
        result.copy_from_slice(&hash);
        result
    }
    
    /// BLAKE3 hash
    pub fn blake3_hash(data: &[u8]) -> [u8; 32] {
        blake3::hash(data).into()
    }
    
    /// Verify Dilithium signature
    pub fn dilithium_verify(
        message: &[u8],
        signature: &[u8],
        public_key: &[u8],
    ) -> Result<bool> {
        use lattice_crypto::verify;
        use lattice_crypto::PublicKey as DilithiumPK;
        use lattice_crypto::Signature as DilithiumSig;
        
        let pk = DilithiumPK::from_bytes(public_key)
            .map_err(|_| OpcodeError::CryptoError("invalid public key".to_string()))?;
        let sig = DilithiumSig::from_bytes(signature)
            .map_err(|_| OpcodeError::CryptoError("invalid signature".to_string()))?;
        
        Ok(verify(message, &sig, &pk).is_ok())
    }
    
    /// Verifiable Random Function (VRF) verification
    pub fn vrf_verify(
        public_key: &[u8],
        proof: &[u8],
        message: &[u8],
    ) -> Result<([u8; 32], bool)> {
        // Simplified VRF verification
        // In production, use proper VRF library like ed25519-vrf
        
        // Extract claimed output from proof
        if proof.len() < 32 {
            return Err(OpcodeError::CryptoError("invalid proof".to_string()));
        }
        
        let mut output = [0u8; 32];
        output.copy_from_slice(&proof[..32]);
        
        // Verify proof (simplified)
        let mut verify_data = Vec::new();
        verify_data.extend_from_slice(public_key);
        verify_data.extend_from_slice(message);
        verify_data.extend_from_slice(&output);
        
        let expected_proof = sha3_256_op(&verify_data);
        let is_valid = &proof[32..] == &expected_proof[..std::cmp::min(proof.len() - 32, 32)];
        
        Ok((output, is_valid))
    }
}

/// Bitwise operations
pub mod bitwise_ops {
    use super::*;
    
    /// Rotate left
    pub fn rotate_left(value: u64, shift: u32) -> u64 {
        value.rotate_left(shift)
    }
    
    /// Rotate right
    pub fn rotate_right(value: u64, shift: u32) -> u64 {
        value.rotate_right(shift)
    }
    
    /// Count set bits (population count)
    pub fn pop_count(value: u64) -> u32 {
        value.count_ones()
    }
    
    /// Count leading zeros
    pub fn leading_zeros(value: u64) -> u32 {
        value.leading_zeros()
    }
    
    /// Count trailing zeros
    pub fn trailing_zeros(value: u64) -> u32 {
        value.trailing_zeros()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use super::math_ops::*;
    use super::crypto_ops::*;
    use super::bitwise_ops::*;
    
    #[test]
    fn test_math_exp() {
        assert_eq!(exp(2, 10).unwrap(), 1024);
        assert_eq!(exp(5, 3).unwrap(), 125);
        assert!(exp(u64::MAX, 2).is_err()); // Overflow
    }
    
    #[test]
    fn test_math_gcd_lcm() {
        assert_eq!(gcd(48, 18), 6);
        assert_eq!(lcm(4, 6).unwrap(), 12);
    }
    
    #[test]
    fn test_math_mod_exp() {
        assert_eq!(mod_exp(2, 10, 1000).unwrap(), 24);
        assert_eq!(mod_exp(3, 5, 7).unwrap(), 5);
    }
    
    #[test]
    fn test_bitwise() {
        assert_eq!(rotate_left(0b1010, 1), 0b10100);
        assert_eq!(pop_count(0b1010), 2);
        assert_eq!(leading_zeros(0b1010), 60);
    }
    
    #[test]
    fn test_crypto_hash() {
        let data = b"test data";
        let hash = sha3_256_op(data);
        assert_eq!(hash.len(), 32);
        
        let blake3 = blake3_hash(data);
        assert_eq!(blake3.len(), 32);
    }
    
    #[test]
    fn test_vm_stack() {
        let mut stack = VmStack::new(10);
        
        stack.push(42).unwrap();
        stack.push(100).unwrap();
        
        assert_eq!(stack.depth(), 2);
        assert_eq!(stack.pop().unwrap(), 100);
        assert_eq!(stack.pop().unwrap(), 42);
        assert!(stack.pop().is_err());
    }
}
