# Lattice Architecture

## Overview

Lattice is structured as a Cargo workspace with multiple crates, each handling a specific concern. This document describes the high-level architecture and data flow.

## Component Diagram

```
┌─────────────────────────────────────────────────────────────────┐
│                         APPLICATIONS                             │
├──────────────────┬──────────────────┬───────────────────────────┤
│   lattice-node   │   lattice-cli    │     lattice-miner         │
│   (Full Node)    │   (Wallet CLI)   │   (Mining Client)         │
└────────┬─────────┴────────┬─────────┴───────────┬───────────────┘
         │                  │                     │
         ▼                  ▼                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                         SERVICES                                 │
├─────────────────┬─────────────────┬─────────────────────────────┤
│  lattice-rpc    │ lattice-network │    lattice-consensus        │
│  (JSON-RPC)     │  (P2P Layer)    │    (PoW Engine)             │
└────────┬────────┴────────┬────────┴───────────┬─────────────────┘
         │                 │                    │
         ▼                 ▼                    ▼
┌─────────────────────────────────────────────────────────────────┐
│                          CORE                                    │
├──────────────────┬──────────────────┬───────────────────────────┤
│ lattice-storage  │   lattice-vm     │    lattice-wallet         │
│ (RocksDB)        │   (WASM)         │    (Keys/Signing)         │
└────────┬─────────┴────────┬─────────┴───────────┬───────────────┘
         │                  │                     │
         ▼                  ▼                     ▼
┌─────────────────────────────────────────────────────────────────┐
│                       PRIMITIVES                                 │
├─────────────────────────────┬───────────────────────────────────┤
│        lattice-core         │        lattice-crypto             │
│  (Block, Tx, State, Addr)   │  (Dilithium, Kyber, SHA3)        │
└─────────────────────────────┴───────────────────────────────────┘
```

## Data Flow

### Transaction Lifecycle

```
1. User creates transaction
   └─► lattice-wallet: TransactionBuilder
       └─► Sign with Dilithium keypair
   
2. Submit to network
   └─► lattice-rpc: lat_sendRawTransaction
       └─► Validate signature & balance
       └─► Add to mempool
   
3. Propagate to peers
   └─► lattice-network: GossipSub publish
       └─► All peers receive and validate
   
4. Include in block
   └─► lattice-consensus: Miner
       └─► Select from mempool by fee
       └─► Build block template
   
5. Mine block
   └─► PoW: Find nonce where hash < target
       └─► Memory-hard computation (Argon2)
   
6. Broadcast block
   └─► lattice-network: GossipSub publish
       └─► Peers validate and apply
   
7. Update state
   └─► lattice-storage: Apply block
       └─► Update account balances
       └─► Remove txs from mempool
```

### Block Synchronization

```
1. New peer connects
   └─► lattice-network: Handshake
       └─► Exchange chain tips
   
2. Identify missing blocks
   └─► Compare block heights
       └─► Request headers first
   
3. Download headers
   └─► Validate PoW difficulty
       └─► Build header chain
   
4. Download blocks
   └─► Parallel block fetching
       └─► Validate transactions
   
5. Apply blocks
   └─► lattice-storage: Sequential application
       └─► Update state root
```

## State Management

### Account Model

```
Account {
    balance: u128,      // Token balance
    nonce: u64,         // Transaction count
    code_hash: [u8;32], // Contract code (if any)
    storage_root: Hash, // Contract storage root
}
```

### State Storage

- Uses RocksDB with column families:
  - `blocks` - Block data by hash
  - `block_index` - Hash by height
  - `state` - Account data by address
  - `code` - Contract bytecode
  - `storage` - Contract storage

### State Transitions

```rust
fn apply_transaction(state: &mut State, tx: &Transaction) -> Result<()> {
    // 1. Verify signature
    verify_signature(&tx)?;
    
    // 2. Check nonce
    let sender = state.get_account(&tx.from);
    ensure!(sender.nonce == tx.nonce);
    
    // 3. Check balance
    let cost = tx.amount + tx.fee;
    ensure!(sender.balance >= cost);
    
    // 4. Deduct from sender
    state.transfer(&tx.from, &tx.to, tx.amount)?;
    state.sub_balance(&tx.from, tx.fee);
    
    // 5. Execute contract (if applicable)
    if tx.kind == TransactionKind::Call {
        execute_contract(state, tx)?;
    }
    
    // 6. Increment nonce
    state.increment_nonce(&tx.from);
    
    Ok(())
}
```

## Network Protocol

### Message Types

| Type | Direction | Description |
|------|-----------|-------------|
| `Status` | Both | Chain tip exchange |
| `NewBlock` | Broadcast | Announce new block |
| `NewTransaction` | Broadcast | Announce new tx |
| `GetHeaders` | Request | Request header range |
| `Headers` | Response | Header batch |
| `GetBlocks` | Request | Request block bodies |
| `Blocks` | Response | Block batch |

### GossipSub Topics

- `/lattice/1/blocks` - New block announcements
- `/lattice/1/transactions` - New transaction announcements

## Smart Contract Execution

### WASM Runtime

```
┌─────────────────────────────────────┐
│           Host Environment          │
│  ┌─────────────────────────────┐   │
│  │        GasMeter             │   │
│  └─────────────────────────────┘   │
│  ┌─────────────────────────────┐   │
│  │     Host Functions          │   │
│  │  - storage_read/write       │   │
│  │  - get_caller               │   │
│  │  - get_block_number         │   │
│  │  - sha3                     │   │
│  │  - emit_event               │   │
│  └─────────────────────────────┘   │
│              │                      │
│              ▼                      │
│  ┌─────────────────────────────┐   │
│  │    WASM Module (Contract)   │   │
│  │    - Linear Memory          │   │
│  │    - Function Exports       │   │
│  └─────────────────────────────┘   │
└─────────────────────────────────────┘
```

### Gas Metering

Gas is charged for:
- Each WASM opcode (weighted by cost)
- Host function calls
- Memory allocation
- Storage operations

Execution stops when gas exhausted, with state rolled back.

## Security Considerations

### Quantum Resistance

All cryptographic operations use post-quantum algorithms:
- **Signatures**: Dilithium3 (3,293 byte signatures)
- **Key Exchange**: Kyber768 (P2P encryption)
- **Hashing**: SHA3-256

### Consensus Security

- 51% attack requires majority of memory bandwidth
- Difficulty adjustment prevents timestamp manipulation
- Checkpoints for finality (optional)

### Smart Contract Security

- WASM sandboxing prevents host access
- Gas limits prevent infinite loops
- Deterministic execution (no floats, no randomness)
