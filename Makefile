# Lattice Blockchain - Makefile
# Convenient commands for development and deployment

.PHONY: all build test clean install run dev fmt lint bench docs docker help

# Default target
all: build

# Build all binaries in release mode
build:
	@echo "🔨 Building Lattice..."
	cargo build --release --bins
	@echo "✓ Build complete!"

# Build for development (faster, with debug info)
dev:
	@echo "🔨 Building for development..."
	cargo build --bins
	@echo "✓ Development build complete!"

# Run tests
test:
	@echo "🧪 Running tests..."
	cargo test --all --all-features --no-fail-fast
	@echo "✓ All tests passed!"

# Run tests with output
test-verbose:
	@echo "🧪 Running tests (verbose)..."
	cargo test --all --all-features --no-fail-fast -- --nocapture
	@echo "✓ All tests passed!"

# Run specific crate tests
test-core:
	cargo test -p lattice-core

test-crypto:
	cargo test -p lattice-crypto

test-consensus:
	cargo test -p lattice-consensus

test-network:
	cargo test -p lattice-network

# Format code
fmt:
	@echo "🎨 Formatting code..."
	cargo fmt --all
	@echo "✓ Code formatted!"

# Run linter
lint:
	@echo "🔍 Running clippy..."
	cargo clippy --all --all-targets --all-features -- -D warnings
	@echo "✓ Linting complete!"

# Check code (fast)
check:
	@echo "🔍 Checking code..."
	cargo check --all --all-features
	@echo "✓ Check complete!"

# Run benchmarks
bench:
	@echo "⚡ Running benchmarks..."
	cargo bench --all
	@echo "✓ Benchmarks complete!"

# Generate documentation
docs:
	@echo "📚 Generating documentation..."
	cargo doc --all --no-deps --open
	@echo "✓ Documentation generated!"

# Clean build artifacts
clean:
	@echo "🧹 Cleaning build artifacts..."
	cargo clean
	rm -rf target
	@echo "✓ Clean complete!"

# Install binaries to ~/.local/bin (or $HOME/.cargo/bin)
install:
	@echo "📦 Installing Lattice..."
	cargo install --path bins/lattice-node --force
	cargo install --path bins/lattice-cli --force
	cargo install --path bins/lattice-miner --force
	@echo "✓ Installation complete!"
	@echo "Binaries installed:"
	@echo "  • lattice-node"
	@echo "  • lattice-cli"
	@echo "  • lattice-miner"

# Run node in development mode
run-node:
	@echo "🚀 Starting Lattice node..."
	cargo run --bin lattice-node -- --dev

# Run node with config
run-node-config:
	@echo "🚀 Starting Lattice node with config..."
	cargo run --release --bin lattice-node -- --config config/node.toml

# Run miner
run-miner:
	@echo "⛏️ Starting miner..."
	cargo run --release --bin lattice-miner -- --threads 4

# Start local testnet (5 nodes)
testnet:
	@echo "🌐 Starting local testnet..."
	./start-testnet.sh

# Build Docker images
docker:
	@echo "🐳 Building Docker image..."
	docker build -t lattice-node:latest .
	@echo "✓ Docker image built!"

# Build Docker image with specific tag
docker-tag:
	@echo "🐳 Building Docker image with tag..."
	docker build -t lattice-node:$(TAG) .
	@echo "✓ Docker image built with tag: $(TAG)"

# Run Docker container
docker-run:
	@echo "🐳 Running Docker container..."
	docker run -d -p 8545:8545 -p 30333:30333 --name lattice-node lattice-node:latest

# Docker Compose up
compose-up:
	@echo "🐳 Starting Docker Compose..."
	docker-compose up -d
	@echo "✓ Services started!"

# Docker Compose down
compose-down:
	@echo "🐳 Stopping Docker Compose..."
	docker-compose down
	@echo "✓ Services stopped!"

# Update dependencies
update:
	@echo "📦 Updating dependencies..."
	cargo update
	@echo "✓ Dependencies updated!"

# Security audit
audit:
	@echo "🔒 Running security audit..."
	cargo audit
	@echo "✓ Security audit complete!"

# Count lines of code
loc:
	@echo "📊 Lines of code:"
	@find crates bins -name "*.rs" | xargs wc -l | tail -1

# Generate coverage report
coverage:
	@echo "📊 Generating coverage report..."
	cargo tarpaulin --all --all-features --out Html --output-dir coverage
	@echo "✓ Coverage report generated in coverage/"

# Quick development cycle (format, check, test)
quick: fmt check test
	@echo "✓ Quick check passed!"

# Full CI pipeline (format, lint, test, build)
ci: fmt lint test build
	@echo "✓ CI pipeline passed!"

# Create release build with optimizations
release:
	@echo "🚀 Creating release build..."
	cargo build --release --bins
	strip target/release/lattice-node
	strip target/release/lattice-cli
	strip target/release/lattice-miner
	@echo "✓ Release build complete!"
	@ls -lh target/release/lattice-*

# Initialize development environment
init:
	@echo "🔧 Initializing development environment..."
	rustup component add rustfmt clippy
	cargo install cargo-audit cargo-tarpaulin
	@echo "✓ Development environment ready!"

# Show help
help:
	@echo ""
	@echo "╔══════════════════════════════════════════════════════════╗"
	@echo "║          Lattice Blockchain - Makefile Help             ║"
	@echo "╚══════════════════════════════════════════════════════════╝"
	@echo ""
	@echo "Build Commands:"
	@echo "  make build          Build all binaries (release mode)"
	@echo "  make dev            Build for development (debug mode)"
	@echo "  make release        Create optimized release build"
	@echo ""
	@echo "Testing Commands:"
	@echo "  make test           Run all tests"
	@echo "  make test-verbose   Run tests with output"
	@echo "  make test-core      Test lattice-core only"
	@echo "  make bench          Run benchmarks"
	@echo "  make coverage       Generate test coverage report"
	@echo ""
	@echo "Code Quality:"
	@echo "  make fmt            Format code"
	@echo "  make lint           Run clippy linter"
	@echo "  make check          Quick compilation check"
	@echo "  make audit          Run security audit"
	@echo ""
	@echo "Development:"
	@echo "  make run-node       Run node in dev mode"
	@echo "  make run-miner      Run miner"
	@echo "  make testnet        Start local testnet"
	@echo "  make quick          Fast check (fmt + check + test)"
	@echo ""
	@echo "Docker:"
	@echo "  make docker         Build Docker image"
	@echo "  make docker-run     Run Docker container"
	@echo "  make compose-up     Start with Docker Compose"
	@echo "  make compose-down   Stop Docker Compose"
	@echo ""
	@echo "Installation:"
	@echo "  make install        Install binaries"
	@echo "  make init           Setup dev environment"
	@echo ""
	@echo "Utilities:"
	@echo "  make docs           Generate documentation"
	@echo "  make clean          Remove build artifacts"
	@echo "  make update         Update dependencies"
	@echo "  make loc            Count lines of code"
	@echo "  make ci             Run full CI pipeline"
	@echo ""
