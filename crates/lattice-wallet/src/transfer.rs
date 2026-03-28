//! Transaction builder for creating and signing transactions

use lattice_core::{Address, Amount, Network, Transaction, TransactionKind};

use crate::{Result, WalletAccount, WalletError};

/// Default gas limit for simple transfers
const DEFAULT_TRANSFER_GAS: u64 = 21000;
/// Default gas limit for contract calls
const DEFAULT_CALL_GAS: u64 = 100000;
/// Default gas limit for contract deployment
const DEFAULT_DEPLOY_GAS: u64 = 1000000;

/// Builder for constructing and signing transactions
#[derive(Debug, Clone)]
pub struct TransactionBuilder {
    kind: TransactionKind,
    from: Option<Address>,
    to: Option<Address>,
    amount: Amount,
    fee: Amount,
    nonce: Option<u64>,
    data: Vec<u8>,
    gas_limit: Option<u64>,
    chain_id: u32,
}

impl Default for TransactionBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl TransactionBuilder {
    /// Create a new transaction builder
    pub fn new() -> Self {
        Self {
            kind: TransactionKind::Transfer,
            from: None,
            to: None,
            amount: 0,
            fee: 0,
            nonce: None,
            data: Vec::new(),
            gas_limit: None,
            chain_id: Network::Mainnet.chain_id(),
        }
    }

    /// Create a transfer transaction builder
    pub fn transfer() -> Self {
        Self::new().kind(TransactionKind::Transfer)
    }

    /// Create a contract call transaction builder
    pub fn call() -> Self {
        Self::new().kind(TransactionKind::Call)
    }

    /// Create a contract deployment transaction builder
    pub fn deploy() -> Self {
        Self::new().kind(TransactionKind::Deploy)
    }

    /// Set the transaction kind
    pub fn kind(mut self, kind: TransactionKind) -> Self {
        self.kind = kind;
        self
    }

    /// Set the sender address
    pub fn from(mut self, address: Address) -> Self {
        self.from = Some(address);
        self
    }

    /// Set the recipient address
    pub fn to(mut self, address: Address) -> Self {
        self.to = Some(address);
        self
    }

    /// Set the amount to transfer
    pub fn amount(mut self, amount: Amount) -> Self {
        self.amount = amount;
        self
    }

    /// Set the transaction fee
    pub fn fee(mut self, fee: Amount) -> Self {
        self.fee = fee;
        self
    }

    /// Set the nonce
    pub fn nonce(mut self, nonce: u64) -> Self {
        self.nonce = Some(nonce);
        self
    }

    /// Set the transaction data
    pub fn data(mut self, data: Vec<u8>) -> Self {
        self.data = data;
        self
    }

    /// Set the gas limit
    pub fn gas_limit(mut self, gas: u64) -> Self {
        self.gas_limit = Some(gas);
        self
    }

    /// Set the chain ID
    pub fn chain_id(mut self, id: u32) -> Self {
        self.chain_id = id;
        self
    }

    /// Set the network (determines chain ID)
    pub fn network(mut self, network: Network) -> Self {
        self.chain_id = network.chain_id();
        self
    }

    /// Build an unsigned transaction
    pub fn build_unsigned(self) -> Result<Transaction> {
        let default_gas_limit = match self.kind {
            TransactionKind::Transfer => DEFAULT_TRANSFER_GAS,
            TransactionKind::Call => DEFAULT_CALL_GAS,
            TransactionKind::Deploy => DEFAULT_DEPLOY_GAS,
        };
        let from = self.from.ok_or(WalletError::MissingField("from".into()))?;
        let to = self.to.unwrap_or_else(Address::zero);
        let nonce = self.nonce.ok_or(WalletError::MissingField("nonce".into()))?;
        let gas_limit = self.gas_limit.unwrap_or(default_gas_limit);

        Ok(Transaction {
            kind: self.kind,
            from,
            to,
            amount: self.amount,
            fee: self.fee,
            nonce,
            data: self.data,
            gas_limit,
            chain_id: self.chain_id,
            public_key: vec![],
            signature: vec![],
        })
    }

    /// Build and sign the transaction with the given account
    pub fn build(self, account: &mut WalletAccount) -> Result<Transaction> {
        let default_gas_limit = match self.kind {
            TransactionKind::Transfer => DEFAULT_TRANSFER_GAS,
            TransactionKind::Call => DEFAULT_CALL_GAS,
            TransactionKind::Deploy => DEFAULT_DEPLOY_GAS,
        };
        // Use account's address if not set
        let from = self.from.unwrap_or_else(|| account.address().clone());
        
        // Use account's nonce if not set
        let nonce = self.nonce.unwrap_or_else(|| account.next_nonce());
        
        let to = self.to.unwrap_or_else(Address::zero);
        let gas_limit = self.gas_limit.unwrap_or(default_gas_limit);

        let mut tx = Transaction {
            kind: self.kind,
            from,
            to,
            amount: self.amount,
            fee: self.fee,
            nonce,
            data: self.data,
            gas_limit,
            chain_id: self.chain_id,
            public_key: account.public_key_bytes(),
            signature: vec![],
        };

        // Sign the transaction
        let signing_bytes = tx.signing_bytes();
        tx.signature = account.sign(&signing_bytes);

        Ok(tx)
    }

    /// Sign an existing unsigned transaction
    pub fn sign(tx: &Transaction, account: &WalletAccount) -> Result<Transaction> {
        if tx.is_signed() {
            return Err(WalletError::Transaction("transaction already signed".into()));
        }

        let mut signed_tx = tx.clone();
        signed_tx.public_key = account.public_key_bytes();
        
        let signing_bytes = signed_tx.signing_bytes();
        signed_tx.signature = account.sign(&signing_bytes);

        Ok(signed_tx)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_build_transfer() {
        let mut account = WalletAccount::generate();
        let recipient = Address::from_bytes([1u8; 20]);

        let tx = TransactionBuilder::transfer()
            .to(recipient.clone())
            .amount(1000)
            .fee(10)
            .build(&mut account)
            .unwrap();

        assert_eq!(tx.kind, TransactionKind::Transfer);
        assert_eq!(tx.to, recipient);
        assert_eq!(tx.amount, 1000);
        assert_eq!(tx.fee, 10);
        assert_eq!(tx.nonce, 0);
        assert!(tx.is_signed());
    }

    #[test]
    fn test_nonce_auto_increment() {
        let mut account = WalletAccount::generate();
        let recipient = Address::from_bytes([1u8; 20]);

        let tx1 = TransactionBuilder::transfer()
            .to(recipient.clone())
            .amount(100)
            .build(&mut account)
            .unwrap();

        let tx2 = TransactionBuilder::transfer()
            .to(recipient.clone())
            .amount(200)
            .build(&mut account)
            .unwrap();

        assert_eq!(tx1.nonce, 0);
        assert_eq!(tx2.nonce, 1);
    }

    #[test]
    fn test_manual_nonce() {
        let mut account = WalletAccount::generate();
        let recipient = Address::from_bytes([1u8; 20]);

        let tx = TransactionBuilder::transfer()
            .to(recipient)
            .amount(100)
            .nonce(42)
            .build(&mut account)
            .unwrap();

        assert_eq!(tx.nonce, 42);
        // Account nonce should not be affected when manually set
    }

    #[test]
    fn test_build_contract_call() {
        let mut account = WalletAccount::generate();
        let contract = Address::from_bytes([2u8; 20]);
        let call_data = vec![0xde, 0xad, 0xbe, 0xef];

        let tx = TransactionBuilder::call()
            .to(contract.clone())
            .data(call_data.clone())
            .gas_limit(50000)
            .build(&mut account)
            .unwrap();

        assert_eq!(tx.kind, TransactionKind::Call);
        assert_eq!(tx.to, contract);
        assert_eq!(tx.data, call_data);
        assert_eq!(tx.gas_limit, 50000);
    }

    #[test]
    fn test_build_deploy() {
        let mut account = WalletAccount::generate();
        let bytecode = vec![0x00, 0x61, 0x73, 0x6d]; // WASM magic

        let tx = TransactionBuilder::deploy()
            .data(bytecode.clone())
            .build(&mut account)
            .unwrap();

        assert_eq!(tx.kind, TransactionKind::Deploy);
        assert!(tx.to.is_zero());
        assert_eq!(tx.data, bytecode);
        assert_eq!(tx.gas_limit, DEFAULT_DEPLOY_GAS);
    }

    #[test]
    fn test_network_chain_id() {
        let mut account = WalletAccount::generate();

        let tx = TransactionBuilder::transfer()
            .to(Address::zero())
            .nonce(0)
            .network(Network::Testnet)
            .build(&mut account)
            .unwrap();

        assert_eq!(tx.chain_id, Network::Testnet.chain_id());
    }

    #[test]
    fn test_build_unsigned() {
        let from = Address::from_bytes([1u8; 20]);
        let to = Address::from_bytes([2u8; 20]);

        let tx = TransactionBuilder::transfer()
            .from(from.clone())
            .to(to.clone())
            .amount(500)
            .nonce(5)
            .build_unsigned()
            .unwrap();

        assert!(!tx.is_signed());
        assert_eq!(tx.from, from);
        assert_eq!(tx.to, to);
        assert_eq!(tx.amount, 500);
        assert_eq!(tx.nonce, 5);
    }

    #[test]
    fn test_sign_unsigned_transaction() {
        let account = WalletAccount::generate();
        let from = account.address().clone();
        let to = Address::from_bytes([2u8; 20]);

        let unsigned_tx = TransactionBuilder::transfer()
            .from(from)
            .to(to)
            .amount(100)
            .nonce(0)
            .build_unsigned()
            .unwrap();

        let signed_tx = TransactionBuilder::sign(&unsigned_tx, &account).unwrap();
        assert!(signed_tx.is_signed());
    }

    #[test]
    fn test_missing_from() {
        let result = TransactionBuilder::transfer()
            .to(Address::zero())
            .nonce(0)
            .build_unsigned();

        assert!(matches!(result, Err(WalletError::MissingField(_))));
    }

    #[test]
    fn test_missing_nonce() {
        let result = TransactionBuilder::transfer()
            .from(Address::zero())
            .to(Address::zero())
            .build_unsigned();

        assert!(matches!(result, Err(WalletError::MissingField(_))));
    }
}
