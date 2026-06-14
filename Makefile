# Lattice Blockchain - Makefile
# Unified workflow around the official `lattice` executable.

.PHONY: all build test clean install run-node run-miner fmt lint bench docs docker help check release quick ci

all: build

build:
	@echo "🔨 Building Lattice unified binary..."
	cargo build --release --bin lattice
	@echo "✓ Build complete!"

# Development build
dev:
	@echo "🔨 Building development binary..."
	cargo build --bin lattice
	@echo "✓ Development build complete!"

test:
	@echo "🧪 Running tests..."
	cargo test --all --all-features --no-fail-fast
	@echo "✓ All tests passed!"

test-verbose:
	@echo "🧪 Running tests (verbose)..."
	cargo test --all --all-features --no-fail-fast -- --nocapture
	@echo "✓ All tests passed!"

fmt:
	@echo "🎨 Formatting code..."
	cargo fmt --all
	@echo "✓ Code formatted!"

lint:
	@echo "🔍 Running clippy..."
	cargo clippy --all --all-targets --all-features -- -D warnings
	@echo "✓ Linting complete!"

check:
	@echo "🔍 Checking code..."
	cargo check --all --all-features
	@echo "✓ Check complete!"

bench:
	@echo "⚡ Running benchmarks..."
	cargo bench --all
	@echo "✓ Benchmarks complete!"

docs:
	@echo "📚 Generating documentation..."
	cargo doc --all --no-deps --open
	@echo "✓ Documentation generated!"

clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	rm -rf target
	@echo "✓ Clean complete!"

install:
	@echo "📦 Installing unified Lattice CLI..."
	cargo install --path bins/lattice --force
	@echo "✓ Installation complete!"
	@echo "Installed binary: lattice"
	@echo "Legacy wrappers (lattice-node / lattice-cli / lattice-miner) are compatibility-only."

run-node:
	@echo "🚀 Starting unified node..."
	cargo run --bin lattice -- node --network devnet

run-miner:
	@echo "⛏️ Starting unified miner..."
	cargo run --bin lattice -- miner --coinbase <ADDRESS> --threads 4 --network devnet

snapshot:
	@echo "📊 Showing unified snapshot..."
	cargo run --bin lattice --

testnet:
	@echo "🌐 Starting local testnet..."
	./start-testnet.sh

docker:
	@echo "🐳 Building unified Docker image..."
	docker build -t lattice:latest .
	@echo "✓ Docker image built!"

docker-run:
	@echo "🐳 Running Docker container..."
	docker run -d -p 8545:8545 -p 30303:30303 --name lattice lattice:latest

compose-up:
	@echo "🐳 Starting Docker Compose..."
	docker-compose up -d
	@echo "✓ Services started!"

compose-down:
	@echo "🐳 Stopping Docker Compose..."
	docker-compose down
	@echo "✓ Services stopped!"

quick: fmt check test
	@echo "✓ Quick check passed!"

ci: fmt lint test build
	@echo "✓ CI pipeline passed!"

release:
	@echo "🚀 Creating release build..."
	cargo build --release --bin lattice
	strip target/release/lattice || true
	@echo "✓ Release build complete!"
	@ls -lh target/release/lattice*

help:
	@echo ""
	@echo "╔══════════════════════════════════════════════════════════╗"
	@echo "║          Lattice Blockchain - Unified Makefile          ║"
	@echo "╚══════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "Build Commands:"
	@echo "  make build          Build unified lattice binary (release)"
	@echo "  make dev            Build unified lattice binary (debug)"
	@echo "  make release        Create optimized release build"
	@echo ""
	@echo "Testing Commands:"
	@echo "  make test           Run all tests"
	@echo "  make test-verbose   Run tests with output"
	@echo "  make bench          Run benchmarks"
	@echo ""
	@echo "Code Quality:"
	@echo "  make fmt            Format code"
	@echo "  make lint           Run clippy linter"
	@echo "  make check          Quick compilation check"
	@echo ""
	@echo "Development:"
	@echo "  make snapshot       Show default lattice snapshot"
	@echo "  make run-node       Run unified node"
	@echo "  make run-miner      Run unified miner"
	@echo "  make testnet        Start local testnet"
	@echo ""
	@echo "Installation:"
	@echo "  make install        Install unified lattice binary"
	@echo ""
