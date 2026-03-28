# Lattice
<<<<<<< HEAD

A quantum-resistant blockchain written in Rust.

## Features

- 🔐 **Post-Quantum Security**: CRYSTALS-Dilithium signatures, CRYSTALS-Kyber key exchange
- ⛏️ **CPU-Friendly Mining**: Memory-hard PoW algorithm (ASIC-resistant)
- 🌐 **Decentralized**: libp2p-based P2P networking
- 📜 **Smart Contracts**: WebAssembly runtime
- 💰 **Wallet**: Full key management and transaction building

## Prerequisites

- Rust 1.75+
- RocksDB dependencies (see [RocksDB installation](https://github.com/rust-rocksdb/rust-rocksdb#requirements))

## Building

```bash
# Build all components
cargo build --release

# Build specific binary
cargo build -p lattice-node --release
```

## Testing

```bash
# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p lattice-core

# Run a single test
cargo test -p lattice-core test_block_hash

# Run tests with output
cargo test -- --nocapture

# Run integration tests
cargo test --test '*'
```

## Running

```bash
# Start a node
cargo run -p lattice-node -- --datadir ~/.lattice

# CLI wallet
cargo run -p lattice-cli -- wallet create

# Start mining
cargo run -p lattice-miner -- --threads 4
```

## Project Structure

```
lattice/
├── crates/           # Library crates
│   ├── lattice-core/       # Core types (Block, Transaction, etc.)
│   ├── lattice-crypto/     # Post-quantum cryptography
│   ├── lattice-consensus/  # PoW consensus engine
│   ├── lattice-network/    # P2P networking
│   ├── lattice-vm/         # WASM smart contract VM
│   ├── lattice-storage/    # RocksDB storage layer
│   ├── lattice-rpc/        # JSON-RPC API
│   └── lattice-wallet/     # Wallet library
├── bins/             # Binary crates
│   ├── lattice-node/       # Full node
│   ├── lattice-cli/        # CLI tool
│   └── lattice-miner/      # Standalone miner
└── tests/            # Integration & E2E tests
```

## License

MIT OR Apache-2.0
=======
Lattice the Quantum resistant blockchain
>>>>>>> ffdb41948a62cff8625ce39ce1845afd11645c85
