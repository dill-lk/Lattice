# Lattice: Quantum-Resistant Blockchain

## Overview

**Lattice** is a full-featured, quantum-resistant blockchain written in Rust. It uses post-quantum cryptographic algorithms while maintaining accessible mining on consumer hardware.

### Key Features
- 🔐 **Post-Quantum Security**: Lattice-based cryptography (CRYSTALS-Dilithium for signatures, CRYSTALS-Kyber for key exchange)
- ⛏️ **Accessible Mining**: PoW algorithm optimized for CPUs (memory-hard, ASIC-resistant)
- 🌐 **P2P Networking**: libp2p-based decentralized network layer
- 📜 **Smart Contracts**: WebAssembly (WASM) runtime for deterministic execution
- 💰 **Wallet**: CLI and library for key management and transactions

---

## Architecture

```
┌─────────────────────────────────────────────────────────────┐
│                        Lattice Node                         │
├─────────────────────────────────────────────────────────────┤
│  ┌─────────┐  ┌─────────┐  ┌─────────┐  ┌─────────────────┐ │
│  │  CLI    │  │  RPC    │  │ Wallet  │  │  Web Interface  │ │
│  │ Client  │  │  API    │  │  Lib    │  │   (optional)    │ │
│  └────┬────┘  └────┬────┘  └────┬────┘  └────────┬────────┘ │
│       │            │            │                │          │
│  ┌────┴────────────┴────────────┴────────────────┴────┐     │
│  │                    Node Core                        │     │
│  ├─────────────────────────────────────────────────────┤     │
│  │  ┌──────────┐ ┌──────────┐ ┌──────────┐ ┌────────┐ │     │
│  │  │Blockchain│ │ Mempool  │ │Consensus │ │ State  │ │     │
│  │  │  Store   │ │          │ │ (PoW)    │ │  DB    │ │     │
│  │  └──────────┘ └──────────┘ └──────────┘ └────────┘ │     │
│  └─────────────────────────────────────────────────────┘     │
│                            │                                 │
│  ┌─────────────────────────┴───────────────────────────┐    │
│  │                 P2P Network Layer                    │    │
│  │              (libp2p / gossipsub)                    │    │
│  └──────────────────────────────────────────────────────┘    │
│                            │                                 │
│  ┌─────────────────────────┴───────────────────────────┐    │
│  │              Cryptography Layer                      │    │
│  │  CRYSTALS-Dilithium | CRYSTALS-Kyber | SHA3-256     │    │
│  └──────────────────────────────────────────────────────┘    │
└─────────────────────────────────────────────────────────────┘
```

---

## Project Structure

```
lattice/
├── Cargo.toml                 # Workspace manifest
├── README.md
├── LICENSE
├── docs/
│   ├── whitepaper.md
│   ├── architecture.md
│   └── api-reference.md
├── crates/
│   ├── lattice-core/          # Core blockchain types & logic
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── block.rs
│   │   │   ├── transaction.rs
│   │   │   ├── chain.rs
│   │   │   └── state.rs
│   │   └── Cargo.toml
│   ├── lattice-crypto/        # Post-quantum cryptography
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── dilithium.rs   # Signatures
│   │   │   ├── kyber.rs       # Key exchange
│   │   │   └── hash.rs        # SHA3
│   │   └── Cargo.toml
│   ├── lattice-consensus/     # PoW consensus engine
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── pow.rs
│   │   │   ├── difficulty.rs
│   │   │   └── miner.rs
│   │   └── Cargo.toml
│   ├── lattice-network/       # P2P networking
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── peer.rs
│   │   │   ├── protocol.rs
│   │   │   └── sync.rs
│   │   └── Cargo.toml
│   ├── lattice-vm/            # WASM smart contract VM
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── runtime.rs
│   │   │   ├── host.rs
│   │   │   └── gas.rs
│   │   └── Cargo.toml
│   ├── lattice-storage/       # Database layer (RocksDB)
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── blocks.rs
│   │   │   ├── state.rs
│   │   │   └── mempool.rs
│   │   └── Cargo.toml
│   ├── lattice-rpc/           # JSON-RPC API
│   │   ├── src/
│   │   │   ├── lib.rs
│   │   │   ├── server.rs
│   │   │   └── handlers.rs
│   │   └── Cargo.toml
│   └── lattice-wallet/        # Wallet library
│       ├── src/
│       │   ├── lib.rs
│       │   ├── account.rs
│       │   ├── keystore.rs
│       │   └── transfer.rs
│       └── Cargo.toml
├── bins/
│   ├── lattice-node/          # Full node binary
│   │   ├── src/main.rs
│   │   └── Cargo.toml
│   ├── lattice-cli/           # CLI wallet & tools
│   │   ├── src/main.rs
│   │   └── Cargo.toml
│   └── lattice-miner/         # Standalone miner
│       ├── src/main.rs
│       └── Cargo.toml
└── tests/
    ├── integration/
    └── e2e/
```

---

## Implementation Plan

### Phase 1: Foundation
| ID | Task | Description |
|----|------|-------------|
| 1 | project-setup | Initialize Rust workspace with all crates |
| 2 | crypto-primitives | Implement post-quantum crypto (Dilithium, Kyber, SHA3) |
| 3 | core-types | Define Block, Transaction, Account, State types |
| 4 | serialization | Binary serialization (borsh/bincode) |

### Phase 2: Blockchain Core
| ID | Task | Description |
|----|------|-------------|
| 5 | storage-layer | RocksDB integration for blocks and state |
| 6 | chain-validation | Block and transaction validation rules |
| 7 | state-machine | Account state transitions |
| 8 | mempool | Transaction pool with prioritization |

### Phase 3: Consensus
| ID | Task | Description |
|----|------|-------------|
| 9 | pow-algorithm | Memory-hard, CPU-friendly PoW (Argon2-based) |
| 10 | difficulty-adjustment | Dynamic difficulty targeting |
| 11 | miner-impl | Multi-threaded mining engine |
| 12 | block-production | Block assembly and propagation |

### Phase 4: Networking
| ID | Task | Description |
|----|------|-------------|
| 13 | p2p-foundation | libp2p setup with peer discovery |
| 14 | gossip-protocol | Block and transaction propagation |
| 15 | chain-sync | Initial block download and sync |
| 16 | peer-management | Connection handling and scoring |

### Phase 5: Smart Contracts
| ID | Task | Description |
|----|------|-------------|
| 17 | wasm-runtime | wasmer/wasmtime integration |
| 18 | host-functions | State access, crypto, logging APIs |
| 19 | gas-metering | Execution cost accounting |
| 20 | contract-deployment | Deploy and call semantics |

### Phase 6: Interface
| ID | Task | Description |
|----|------|-------------|
| 21 | rpc-server | JSON-RPC 2.0 API |
| 22 | wallet-lib | Key generation, signing, transaction building |
| 23 | cli-tool | Node control and wallet operations |
| 24 | documentation | API docs and usage guides |

### Phase 7: Testing & Polish
| ID | Task | Description |
|----|------|-------------|
| 25 | unit-tests | Comprehensive test coverage |
| 26 | integration-tests | Multi-node scenarios |
| 27 | testnet | Public testnet deployment |
| 28 | security-audit | Code review and hardening |

---

## Technical Decisions

### Post-Quantum Cryptography
- **Signatures**: CRYSTALS-Dilithium (NIST standardized, ~2.5KB signatures)
- **Key Exchange**: CRYSTALS-Kyber (for encrypted P2P communication)
- **Hashing**: SHA3-256 (quantum-resistant, NIST standard)

### Mining Algorithm
- **RandomX variant**: Memory-hard, optimized for modern CPUs
- **Target**: 15-second block time
- **ASIC Resistance**: Memory-bound computation prevents specialized hardware advantage

### Smart Contract VM
- **WebAssembly**: Portable, sandboxed, efficient
- **Runtime**: wasmer or wasmtime
- **Languages**: Rust, AssemblyScript compile to WASM

### Storage
- **RocksDB**: Fast key-value store for blocks and state
- **Merkle Patricia Trie**: State tree for efficient proofs

---

## Dependencies (Key Crates)

```toml
# Cryptography
pqcrypto = "0.17"              # Post-quantum crypto
pqcrypto-dilithium = "0.5"
pqcrypto-kyber = "0.8"
sha3 = "0.10"

# Networking
libp2p = "0.53"

# Storage
rocksdb = "0.21"

# Smart Contracts
wasmer = "4.2"

# Serialization
serde = "1.0"
borsh = "1.3"

# Async Runtime
tokio = "1.35"

# CLI
clap = "4.4"
```

---

## Network Parameters

| Parameter | Value |
|-----------|-------|
| Block Time | 15 seconds |
| Block Size Limit | 2 MB |
| Target TPS | ~100 |
| Mining Memory | 4 GB recommended |
| Address Format | Base58Check (Dilithium pubkey derived) |

---

## Getting Started (After Implementation)

```bash
# Build all components
cargo build --release

# Generate a new wallet
lattice-cli wallet create

# Start a full node
lattice-node --datadir ~/.lattice --network mainnet

# Start mining
lattice-miner --threads 4 --coinbase <your-address>

# Check balance
lattice-cli wallet balance

# Send transaction
lattice-cli tx send --to <address> --amount 100
```

---

## Status

🔴 **Not Started** - Ready to begin implementation

When ready, say **"start"** or **"let's build"** to begin Phase 1!
