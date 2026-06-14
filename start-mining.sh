#!/bin/bash
# Unified quick-start mining script for Lattice

set -e

GREEN='\033[0;32m'
CYAN='\033[0;36m'
YELLOW='\033[1;33m'
NC='\033[0m'

DEFAULT_WALLET="wallet.json"

echo -e "${CYAN}"
echo "╔══════════════════════════════════════════════════════════╗"
echo "║        ⛏️  LATTICE QUICK START MINING (UNIFIED)         ║"
echo "╚══════════════════════════════════════════════════════════╝"
echo -e "${NC}\n"

if ! command -v lattice &> /dev/null; then
    echo -e "${YELLOW}Lattice not found. Install with:${NC}"
    echo "  curl -fsSL https://raw.githubusercontent.com/dill-lk/Lattice/main/install.sh | bash"
    exit 1
fi

echo -e "${GREEN}✓${NC} unified lattice CLI found\n"

if [ ! -f "$DEFAULT_WALLET" ]; then
    echo -e "${YELLOW}No local default wallet found. Creating one...${NC}"
    lattice --wallet-new
    echo ""
fi

echo -e "${GREEN}✓${NC} wallet ready\n"

WALLET_ADDR=$(lattice --json wallet address --wallet "$DEFAULT_WALLET" | grep '"address"' | head -1 | sed 's/.*: *"\([^"]*\)".*/\1/')

if [ -n "$WALLET_ADDR" ]; then
    echo -e "${CYAN}Mining to:${NC} $WALLET_ADDR\n"
fi

if command -v nproc &> /dev/null; then
    CORES=$(nproc)
elif command -v sysctl &> /dev/null; then
    CORES=$(sysctl -n hw.ncpu)
else
    CORES=4
fi

THREADS=$((CORES * 3 / 4))
if [ $THREADS -lt 1 ]; then
    THREADS=1
fi

echo -e "${CYAN}CPU Cores detected:${NC} $CORES"
echo -e "${CYAN}Mining threads:${NC} $THREADS\n"

echo -e "${GREEN}Starting unified local mining...${NC}"
echo -e "${CYAN}This path will auto-start local integrated miner-node mode if needed.${NC}\n"

lattice --mine "$THREADS"
