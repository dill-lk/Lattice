# ⛏️ Lattice Project Context

Lattice is a production-ready, quantum-resistant blockchain designed for CPU-friendly mining and secure, future-proof decentralized applications.

## 🏗️ Architecture Overview

Lattice is structured as a modular Rust workspace:

### Binaries (`bins/`)
- **`lattice-node`**: The core full node implementation. Handles peer discovery, block synchronization, state management, and the RPC server.
- **`lattice-cli`**: A command-line tool for interacting with the blockchain (wallet management, querying balance, node status).
- **`lattice-miner`**: A dedicated mining client that performs the memory-hard PoW search.

### Core Libraries (`crates/`)
- **`lattice-crypto`**: **Quantum-resistant primitives**. Uses CRYSTALS-Dilithium3 for signatures, Kyber768 for key encapsulation (KEM), and SHA3-256 for hashing.
- **`lattice-core`**: Fundamental blockchain types: Blocks, Transactions, Addresses, and State transitions.
- **`lattice-consensus`**: The PoW engine. Uses **Argon2** for ASIC-resistant, memory-hard hashing.
- **`lattice-network`**: P2P layer built on **libp2p** (GossipSub, Noise, Yamux).
- **`lattice-vm`**: WASM-based smart contract execution environment powered by **Wasmer**.
- **`lattice-storage`**: Persistent storage layer using **RocksDB** for state and block data.
- **`lattice-rpc`**: JSON-RPC server for external client interactions.
- **`lattice-wallet`**: Key management and transaction signing logic.

## 🛠️ Development Guide

### Prerequisites
- **Rust**: Version 1.75+
- **C++ Compiler**: Required for RocksDB and some crypto dependencies.

### Key Commands
- **Build All**: `cargo build --release`
- **Run Tests**: `cargo test --workspace`
- **Start Node/Miner**: `./start-mining.sh` (or `cargo run -p lattice-miner`)
- **Check Wallet**: `cargo run -p lattice-cli -- wallet balance`
- **Linting**: `cargo clippy`
- **Formatting**: `cargo fmt`

### Core Data Structures
- **Address**: 20-byte identifier derived from Dilithium public keys.
- **Block Time**: ~15 seconds target.
- **Reward**: 10 LAT per block.
- **Hashing**: SHA3-256 (32 bytes).

## 📝 Conventions & Standards

- **Async Runtime**: Uses `tokio` for all asynchronous operations.
- **Error Handling**: Prefers `thiserror` for library-level errors and `anyhow` for application-level (bin) errors.
- **Logging**: Uses `tracing` and `tracing-subscriber` for structured logging.
- **Serialization**: 
  - `serde`/`serde_json` for RPC and configuration.
  - `borsh` for efficient binary serialization of blocks and transactions.
- **Documentation**: All public modules and types should be documented with doc comments (`///`).
- **Testing**: Integration tests are located in `tests/integration/`. Unit tests are inline within modules.

## 🔒 Security & Performance

- **Post-Quantum Security**: Never use non-PQ signature schemes. Always use `lattice-crypto` wrappers.
- **ASIC Resistance**: The PoW algorithm is memory-hard. Avoid optimizations that favor high-throughput low-memory hardware.
- **Gas Metering**: Smart contracts MUST be metered. Infinite loops or excessive memory usage in WASM are prevented by `lattice-vm` gas limits.

## 🗺️ Roadmap & Status
Refer to `STATUS.md` and `FINAL_REPORT.md` for current development progress and completed milestones.
