# Lattice - Copilot Instructions

Lattice is a quantum-resistant blockchain written in Rust. It uses post-quantum cryptography (CRYSTALS-Dilithium/Kyber) and a memory-hard PoW algorithm.

## Build & Test Commands

```bash
# Build everything
cargo build --release

# Build specific crate
cargo build -p lattice-core

# Run all tests
cargo test

# Run tests for a specific crate
cargo test -p lattice-core

# Run single test by name
cargo test -p lattice-core test_block_hash

# Run tests with stdout visible
cargo test -- --nocapture

# Run only integration tests
cargo test --test '*'

# Run benchmarks
cargo bench

# Check without building
cargo check --all

# Format code
cargo fmt --all

# Lint
cargo clippy --all -- -D warnings
```

## Architecture

### Crate Dependency Graph

```
                    ┌─────────────────┐
                    │   lattice-node  │ (binary)
                    └────────┬────────┘
                             │
        ┌────────────────────┼────────────────────┐
        │                    │                    │
        ▼                    ▼                    ▼
┌───────────────┐   ┌───────────────┐   ┌───────────────┐
│lattice-network│   │ lattice-rpc   │   │lattice-consensus│
└───────┬───────┘   └───────┬───────┘   └───────┬───────┘
        │                   │                   │
        └─────────────┬─────┴─────────┬─────────┘
                      │               │
                      ▼               ▼
              ┌───────────────┐ ┌───────────────┐
              │lattice-storage│ │  lattice-vm   │
              └───────┬───────┘ └───────┬───────┘
                      │                 │
                      └────────┬────────┘
                               │
                      ┌────────┴────────┐
                      ▼                 ▼
              ┌───────────────┐ ┌───────────────┐
              │ lattice-core  │◄│lattice-crypto │
              └───────────────┘ └───────────────┘
```

### Crate Responsibilities

- **lattice-core**: Fundamental types (`Block`, `Transaction`, `Address`, `State`). All other crates depend on this.
- **lattice-crypto**: Post-quantum cryptography (Dilithium signatures, Kyber key exchange, SHA3 hashing).
- **lattice-consensus**: PoW mining algorithm, difficulty adjustment.
- **lattice-network**: P2P networking with libp2p, gossipsub for propagation, chain sync.
- **lattice-vm**: WASM smart contract runtime with gas metering.
- **lattice-storage**: RocksDB persistence for blocks, state, and mempool.
- **lattice-rpc**: JSON-RPC 2.0 API server.
- **lattice-wallet**: Key management, transaction building, signing.

### Key Types

| Type | Location | Description |
|------|----------|-------------|
| `Block` | `lattice-core::block` | Block with header and transactions |
| `Transaction` | `lattice-core::transaction` | Signed transaction |
| `Address` | `lattice-core::address` | 20-byte address from Dilithium pubkey |
| `State` | `lattice-core::state` | World state (address → account) |
| `Keypair` | `lattice-crypto::dilithium` | Dilithium signing keypair |
| `Hash` | `lattice-crypto::hash` | SHA3-256 hash (32 bytes) |

## Code Conventions

### Error Handling

- Use `thiserror` for error types in library crates
- Use `anyhow` in binary crates
- Each crate defines its own `Error` and `Result` types in `error.rs`

```rust
// Library crate pattern
#[derive(Debug, thiserror::Error)]
pub enum CoreError {
    #[error("invalid block: {0}")]
    InvalidBlock(String),
}
pub type Result<T> = std::result::Result<T, CoreError>;
```

### Serialization

- Use `borsh` for binary serialization (compact, deterministic)
- Use `serde` for JSON/config files
- Types typically derive both:

```rust
#[derive(BorshSerialize, BorshDeserialize, Serialize, Deserialize)]
pub struct Block { ... }
```

### Async Runtime

- Use `tokio` for async (full features enabled)
- Prefer async for I/O-bound operations (networking, storage)
- Mining is CPU-bound and uses standard threads via `rayon` or `std::thread`

### Cryptographic Operations

- All crypto goes through `lattice-crypto` - never use raw pqcrypto directly
- Signatures are Dilithium3 (~2.5KB)
- Key exchange is Kyber768
- Hashing is SHA3-256

### Address Format

- 20 bytes derived from SHA3-256(public_key)[0..20]
- Encoded as Base58Check with version byte
- Zero address (`[0u8; 20]`) is reserved for coinbase/system operations

### Testing

- Unit tests in `#[cfg(test)]` modules within source files
- Integration tests in `tests/` directory
- Use `proptest` for property-based testing of serialization roundtrips
- Benchmarks use `criterion` in `benches/`

## Network Parameters

| Parameter | Value |
|-----------|-------|
| Block time | 15 seconds |
| Block size | 2 MB max |
| Chain ID (mainnet) | 1 |
| Chain ID (testnet) | 2 |
| Base gas | 21,000 |
| Gas per data byte | 16 |
