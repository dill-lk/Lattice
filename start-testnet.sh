#!/bin/bash
# Lattice Blockchain - Quick Start Script
# This script sets up a local testnet for development

set -e

echo "🔗 Lattice Blockchain - Local Testnet Setup"
echo "============================================"

# Colors
RED='\033[0;31m'
GREEN='\033[0;32m'
YELLOW='\033[1;33m'
NC='\033[0m'

# Check if Rust is installed
if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Rust is not installed${NC}"
    echo "Install from: https://rustup.rs/"
    exit 1
fi

echo -e "${GREEN}✓${NC} Rust found: $(rustc --version)"

# Build project
echo ""
echo "📦 Building Lattice (this may take a few minutes)..."
cargo build --release

if [ $? -eq 0 ]; then
    echo -e "${GREEN}✓${NC} Build successful!"
else
    echo -e "${RED}❌ Build failed${NC}"
    exit 1
fi

# Create data directories
echo ""
echo "📁 Creating data directories..."
mkdir -p ~/.lattice/devnet-node1
mkdir -p ~/.lattice/devnet-node2
mkdir -p ~/.lattice/devnet-miner

echo -e "${GREEN}✓${NC} Directories created"

# Create wallet for miner
echo ""
echo "💰 Creating mining wallet..."
./target/release/lattice-cli wallet create --datadir ~/.lattice/devnet-miner

if [ $? -eq 0 ]; then
    COINBASE=$(./target/release/lattice-cli wallet address --datadir ~/.lattice/devnet-miner)
    echo -e "${GREEN}✓${NC} Wallet created"
    echo -e "${YELLOW}Your mining address: $COINBASE${NC}"
else
    echo -e "${RED}❌ Wallet creation failed${NC}"
    exit 1
fi

# Start node 1
echo ""
echo "🚀 Starting Node 1..."
./target/release/lattice-node \
  --datadir ~/.lattice/devnet-node1 \
  --network devnet \
  --rpc-port 9933 \
  --p2p-port 30303 \
  --log-level info &

NODE1_PID=$!
echo -e "${GREEN}✓${NC} Node 1 started (PID: $NODE1_PID)"

# Wait for node to start
echo "⏳ Waiting for node to initialize..."
sleep 5

# Start miner
echo ""
echo "⛏️  Starting Miner..."
./target/release/lattice-miner \
  --threads 2 \
  --coinbase "$COINBASE" \
  --node-rpc http://localhost:9933 \
  --network devnet &

MINER_PID=$!
echo -e "${GREEN}✓${NC} Miner started (PID: $MINER_PID)"

# Display status
echo ""
echo "============================================"
echo -e "${GREEN}✅ LOCAL TESTNET RUNNING${NC}"
echo "============================================"
echo ""
echo "📊 Status:"
echo "  Node 1:  http://localhost:9933"
echo "  P2P:     localhost:30303"
echo "  Miner:   Running on $COINBASE"
echo ""
echo "🔧 Useful commands:"
echo "  Check height:    curl -X POST http://localhost:9933 -H 'Content-Type: application/json' -d '{\"jsonrpc\":\"2.0\",\"method\":\"lat_blockNumber\",\"params\":[],\"id\":1}'"
echo "  Check balance:   ./target/release/lattice-cli query balance $COINBASE"
echo "  Send tx:         ./target/release/lattice-cli tx send --to <address> --amount 1000"
echo ""
echo "⚠️  To stop:"
echo "  kill $NODE1_PID $MINER_PID"
echo ""
echo "📝 Logs:"
echo "  Node:   tail -f ~/.lattice/devnet-node1/node.log"
echo "  Miner:  Check terminal output"
echo ""

# Save PIDs for cleanup script
echo "$NODE1_PID" > ~/.lattice/node1.pid
echo "$MINER_PID" > ~/.lattice/miner.pid

echo "Press Ctrl+C to stop all processes"
echo ""

# Wait for Ctrl+C
trap "echo ''; echo '🛑 Stopping...'; kill $NODE1_PID $MINER_PID 2>/dev/null; echo '✓ Stopped'; exit 0" INT

# Keep script running
wait
