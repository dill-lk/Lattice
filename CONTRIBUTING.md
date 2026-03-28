# Contributing to Lattice

Thank you for your interest in contributing to Lattice!

## Getting Started

1. Fork the repository
2. Clone your fork: `git clone https://github.com/YOUR_USERNAME/lattice.git`
3. Create a branch: `git checkout -b feature/your-feature`
4. Make your changes
5. Run tests: `cargo test`
6. Run lints: `cargo clippy --all -- -D warnings`
7. Format code: `cargo fmt --all`
8. Commit and push
9. Open a Pull Request

## Development Setup

### Prerequisites

- Rust 1.75+ (install via [rustup](https://rustup.rs/))
- RocksDB dependencies:
  - **Linux**: `apt install librocksdb-dev clang`
  - **macOS**: `brew install rocksdb`
  - **Windows**: See [rust-rocksdb docs](https://github.com/rust-rocksdb/rust-rocksdb)

### Building

```bash
# Debug build
cargo build

# Release build  
cargo build --release

# Build specific crate
cargo build -p lattice-core
```

### Testing

```bash
# All tests
cargo test

# Single crate
cargo test -p lattice-core

# Single test
cargo test -p lattice-core test_block_hash

# With output
cargo test -- --nocapture
```

## Code Style

- Follow Rust conventions
- Use `cargo fmt` before committing
- All code must pass `cargo clippy -- -D warnings`
- Write doc comments for public APIs
- Add tests for new functionality

## Commit Messages

Use conventional commits:

```
feat(core): add merkle proof verification
fix(network): handle peer disconnect gracefully
docs: update API reference
test(vm): add gas metering tests
```

## Pull Request Process

1. Update documentation if needed
2. Add tests for new features
3. Ensure CI passes
4. Request review from maintainers
5. Squash commits if requested

## Code of Conduct

Be respectful and inclusive. We follow the [Rust Code of Conduct](https://www.rust-lang.org/policies/code-of-conduct).

## Questions?

Open an issue or join our Discord!
