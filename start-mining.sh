#!/bin/bash
# Quick start mining script for Lattice Blockchain

set -e

# Colors
GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

echo -e "${CYAN}"
echo "╔══════════════════════════════════════════════════════════╗"
echo "║                                                          ║"
echo "║        ⛏️  LATTICE QUICK START MINING ⛏️               ║"
echo "║                                                          ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo -e "${NC}\n"

# Check if lattice is installed
if ! command -v lattice-node &> /dev/null; then
    echo -e "${YELLOW}Lattice not found. Install with:${NC}"
    echo "  curl -sSfL https://latticechain.io/install.sh | bash"
    exit 1
fi

echo -e "${GREEN}✓ Lattice is installed${NC}\n"

# Check for wallet
if [ ! -f "wallet.json" ] && [ ! -f "$HOME/.lattice/wallet.json" ]; then
    echo -e "${YELLOW}No wallet found. Creating one...${NC}"
    lattice-cli wallet create
    echo ""
fi

echo -e "${GREEN}✓ Wallet ready${NC}\n"

# Get wallet address
WALLET_ADDR=$(lattice-cli wallet address 2>/dev/null | grep "lat1" || echo "")

if [ -n "$WALLET_ADDR" ]; then
    echo -e "${CYAN}Mining to:${NC} $WALLET_ADDR\n"
fi

# Detect CPU cores
if command -v nproc &> /dev/null; then
    CORES=$(nproc)
elif command -v sysctl &> /dev/null; then
    CORES=$(sysctl -n hw.ncpu)
else
    CORES=4
fi

# Use 75% of cores
THREADS=$((CORES * 3 / 4))
if [ $THREADS -lt 1 ]; then
    THREADS=1
fi

echo -e "${CYAN}CPU Cores detected:${NC} $CORES"
echo -e "${CYAN}Mining threads:${NC} $THREADS\n"

# Check if node is running
if ! curl -s http://localhost:8545/health &> /dev/null; then
    echo -e "${YELLOW}Node not running. Starting in dev mode...${NC}\n"
    lattice-node --dev &
    sleep 5
    echo -e "${GREEN}✓ Node started${NC}\n"
else
    echo -e "${GREEN}✓ Node is running${NC}\n"
fi

# Start mining
echo -e "${GREEN}Starting miner with $THREADS threads...${NC}\n"
echo -e "${CYAN}╔══════════════════════════════════════════════════════════╗${NC}"
echo -e "${CYAN}║  Press Ctrl+C to stop mining                             ║${NC}"
echo -e "${CYAN}╚══════════════════════════════════════════════════════════╝${NC}\n"

if [ -n "$WALLET_ADDR" ]; then
    lattice-miner --threads $THREADS --address "$WALLET_ADDR"
else
    lattice-miner --threads $THREADS
fi
