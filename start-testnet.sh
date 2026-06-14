#!/bin/bash
# Unified local testnet helper for Lattice

set -e

echo "🔗 Lattice - Local Devnet / Testnet Helper"
echo "==========================================="

GREEN='\033[0;32m'
YELLOW='\033[1;33m'
RED='\033[0;31m'
NC='\033[0m'

if ! command -v cargo &> /dev/null; then
    echo -e "${RED}❌ Rust/Cargo is not installed${NC}"
    echo "Install from: https://rustup.rs/"
    exit 1
fi

echo -e "${GREEN}✓${NC} Rust found: $(rustc --version)"

echo ""
echo "📦 Building unified lattice binary..."
cargo build --release --bin lattice

echo -e "${GREEN}✓${NC} Build successful"

mkdir -p ~/.lattice/devnet-node1

echo ""
echo "💰 Creating / checking default wallet..."
if [ ! -f "wallet.json" ]; then
  ./target/release/lattice --wallet-new
fi
COINBASE=$(./target/release/lattice --json wallet address --wallet wallet.json | grep '"address"' | head -1 | sed 's/.*: *"\([^"]*\)".*/\1/')

echo -e "${GREEN}✓${NC} Wallet ready"
echo -e "${YELLOW}Mining address: $COINBASE${NC}"

echo ""
echo "🚀 Starting unified devnet node..."
./target/release/lattice node \
  --datadir ~/.lattice/devnet-node1 \
  --network devnet \
  --rpc-port 8545 \
  --p2p-port 30303 \
  --log-level info &
NODE1_PID=$!

echo -e "${GREEN}✓${NC} Node started (PID: $NODE1_PID)"
sleep 5

echo ""
echo "⛏️  Starting unified miner..."
./target/release/lattice miner \
  --threads 2 \
  --coinbase "$COINBASE" \
  --rpc http://localhost:8545 \
  --network devnet &
MINER_PID=$!

echo -e "${GREEN}✓${NC} Miner started (PID: $MINER_PID)"

echo ""
echo "==========================================="
echo -e "${GREEN}✅ LOCAL DEVNET RUNNING${NC}"
echo "==========================================="
echo "  Node RPC:   http://localhost:8545"
echo "  P2P:        localhost:30303"
echo "  Coinbase:   $COINBASE"
echo ""
echo "Useful commands:"
echo "  ./target/release/lattice status"
echo "  ./target/release/lattice chain"
echo "  ./target/release/lattice mempool"
echo "  ./target/release/lattice wallet balance wallet.json"
echo ""
echo "To stop:"
echo "  kill $NODE1_PID $MINER_PID"
echo ""

echo "$NODE1_PID" > ~/.lattice/node1.pid
echo "$MINER_PID" > ~/.lattice/miner.pid

trap "echo ''; echo '🛑 Stopping...'; kill $NODE1_PID $MINER_PID 2>/dev/null; echo '✓ Stopped'; exit 0" INT
wait
