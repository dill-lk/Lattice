# ⛏️ Lattice — Quantum-Resistant Blockchain

<div align="center">

![Lattice Logo](https://img.shields.io/badge/⛏️-Lattice-blue?style=for-the-badge)
[![License](https://img.shields.io/badge/license-MIT%2FApache--2.0-green?style=flat-square)](LICENSE)
[![Rust](https://img.shields.io/badge/rust-1.75%2B-orange?style=flat-square&logo=rust)](https://www.rust-lang.org/)
[![GitHub Release](https://img.shields.io/github/v/release/dill-lk/Lattice?style=flat-square)](https://github.com/dill-lk/Lattice/releases)

**CPU-Friendly Mining · Post-Quantum Cryptography · Fair Launch**

[Quick Start](#-quick-start) • [Mining](#%EF%B8%8F-mining) • [Tokenomics](#-tokenomics) • [Documentation](#-documentation)

</div>

---

## 🌟 What is Lattice?

Lattice is a **quantum-resistant blockchain** built for the future. While other cryptocurrencies will become vulnerable when quantum computers arrive, Lattice is secured by **CRYSTALS-Dilithium3** signatures — the same algorithm chosen by NIST for post-quantum security.

### Key Features

| Feature | Description |
|---------|-------------|
| 🔐 **Quantum-Resistant** | CRYSTALS-Dilithium3 signatures, Kyber768 key exchange |
| ⛏️ **CPU-Friendly Mining** | Argon2 memory-hard PoW — no ASICs, no GPUs needed |
| 🚀 **Fast Blocks** | 2-15 second block times depending on network |
| 💰 **Fair Launch** | 95% of supply goes to miners, only 5% genesis allocation |
| 🔓 **Open Source** | MIT/Apache-2.0 dual license |

---

## 🪙 Tokenomics

| Parameter | Value |
|-----------|-------|
| **Symbol** | LAT |
| **Total Supply** | 50,000,000 LAT |
| **Decimals** | 8 (1 LAT = 100,000,000 Latt) |
| **Block Reward** | 10 LAT |
| **Genesis Allocation** | 5% (2.5M LAT with vesting) |
| **Mining Allocation** | 95% (47.5M LAT) |

### Block Times & Difficulty

| Network | Block Time | Initial Difficulty | PoW Memory |
|---------|------------|-------------------|------------|
| **Devnet** | ~2 seconds | 1 | 512 KB |
| **Testnet** | ~5 seconds | 5 | 4 MB |
| **Mainnet** | ~15 seconds | 10 | 64 MB |

> 💡 Lower difficulty = faster block finding. Use devnet for development!

### Genesis Allocation

The 5% founder allocation ensures sustainable development:

- **500,000 LAT** — Immediately available (exchange listings, infrastructure)
- **2,000,000 LAT** — 24-month linear vesting (long-term commitment)

👉 See [TOKENOMICS.md](TOKENOMICS.md) for full details.

---

## 🚀 Quick Start

### Install (60 seconds)

**Linux / macOS:**
```bash
curl -fsSL https://raw.githubusercontent.com/dill-lk/Lattice/main/install.sh | bash
```

**Windows (PowerShell):**
```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
irm https://raw.githubusercontent.com/dill-lk/Lattice/main/install.ps1 | iex
```

This installs three binaries:

| Binary | Purpose |
|--------|---------|
| `lattice-node` | Full node (syncs blockchain, serves RPC) |
| `lattice-miner` | Standalone CPU miner |
| `lattice-cli` | Wallet & command-line tools |

### Create Wallet

```bash
lattice-cli wallet create
# Output: Address: 13jXqXbCSghDF2KgyFQdtw8SvbJvpEyhft
```

### Start Mining

```bash
# Terminal 1 — Start node
lattice-node --network devnet

# Terminal 2 — Start miner
lattice-miner --coinbase <your-address> --network devnet
```

That's it! You're now mining LAT. 🎉

---

## ⛏️ Mining

### Network Selection

Choose your network based on your needs:

```bash
# Development (instant blocks, ~2 sec)
lattice-node --network devnet
lattice-miner --coinbase <addr> --network devnet

# Testing (moderate speed, ~5 sec)
lattice-node --network testnet
lattice-miner --coinbase <addr> --network testnet

# Production (full security, ~15 sec)
lattice-node --network mainnet
lattice-miner --coinbase <addr> --network mainnet
```

### Thread Recommendations

| CPU Cores | Recommended Threads |
|-----------|---------------------|
| 2 | 2 |
| 4 | 3-4 |
| 8 | 6-8 |
| 16+ | 12-16 |

```bash
lattice-miner --coinbase <addr> --threads 8 --network devnet
```

### Mining Display

The miner shows real-time stats:

```
╔═══════════════════════════════════════════════════════════╗
║                 ⛏  LATTICE MINER  v0.1.0                  ║
╠═══════════════════════════════════════════════════════════╣
║  Network : DEVNET                                         ║
║  Threads : 8                                              ║
║  Coinbase: 13jXqXbCSghDF2KgyFQdtw8SvbJvpEyhft             ║
║  Node RPC: http://127.0.0.1:8545                          ║
╚═══════════════════════════════════════════════════════════╝

 ⛏  #42  │  125.3 H/s  (avg 118.7 H/s)  │  1,247 hashes  │  3 found  │  up 2m
```

---

## 💵 Wallet Commands

```bash
# Create new wallet
lattice-cli wallet create

# Show your address
lattice-cli wallet address

# Check balance
lattice-cli wallet balance

# Send LAT
lattice-cli tx send --to <address> --amount 10.5

# Export private key (⚠️ keep safe!)
lattice-cli wallet export
```

---

## 🖥️ System Requirements

| Tier | CPU | RAM | Disk | Expected Hash Rate |
|------|-----|-----|------|-------------------|
| **Minimum** | 2 cores | 4 GB | 20 GB | ~10 H/s |
| **Recommended** | 4+ cores | 8 GB | 50 GB SSD | ~50 H/s |
| **Optimal** | 8+ cores | 16 GB | 100 GB SSD | ~150+ H/s |

> 💡 Lattice uses Argon2 memory-hard PoW. More RAM = better performance.

---

## 🔐 Security

### Backup Your Wallet

```bash
# Your wallet is at ~/.lattice/wallet.json (or ./wallet.json)
cp wallet.json ~/backup/wallet-$(date +%Y%m%d).json
```

**⚠️ WARNING:** If you lose your wallet file and password, your LAT is **gone forever**.

### Post-Quantum Cryptography

Lattice uses algorithms selected by NIST for post-quantum security:

| Component | Algorithm | Security Level |
|-----------|-----------|----------------|
| Signatures | CRYSTALS-Dilithium3 | NIST Level 3 |
| Key Exchange | Kyber768 | NIST Level 3 |
| Hashing | SHA3-256 | 256-bit |
| PoW | Argon2id | Memory-hard |

---

## 🆘 Troubleshooting

| Problem | Solution |
|---------|----------|
| Node won't start | Check port 30303 isn't in use |
| Miner shows 0.00 H/s | Use `--network devnet` for faster hashes |
| RPC connection error | Make sure `lattice-node` is running |
| Invalid coinbase address | Use address from `lattice-cli wallet address` |
| Low hashrate | Increase `--threads`, close background apps |
| Old chain data | Delete data directory and restart (see below) |

### Reset Chain Data

If you need to start fresh (e.g., after updating difficulty settings):

**Windows:**
```powershell
Remove-Item -Recurse $env:LOCALAPPDATA\Lattice
```

**Linux/macOS:**
```bash
rm -rf ~/.local/share/lattice
```

### Node Commands

```bash
# Check node status
lattice-cli node status

# View peers
lattice-cli node peers

# Open firewall (Linux)
sudo ufw allow 30303/tcp
```

---

## 📚 Documentation

| Document | Description |
|----------|-------------|
| [TOKENOMICS.md](TOKENOMICS.md) | Token supply, genesis allocation, vesting |
| [MINING_GUIDE.md](MINING_GUIDE.md) | In-depth mining optimization |
| [ADMIN.md](ADMIN.md) | Full operator guide, systemd setup |
| [DEPLOYMENT.md](DEPLOYMENT.md) | Docker & production deployment |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Build from source, contribute |
| [docs/api-reference.md](docs/api-reference.md) | JSON-RPC API reference |

---

## 🛠️ Build from Source

```bash
# Requirements: Rust 1.75+, C++ compiler (for RocksDB)

git clone https://github.com/dill-lk/Lattice.git
cd Lattice

# Build
cargo build --release

# Test
cargo test --workspace

# Install locally
cargo install --path bins/lattice-node
cargo install --path bins/lattice-miner
cargo install --path bins/lattice-cli
```

---

## 🏗️ Architecture

```
┌─────────────────┐
│   lattice-node  │ ← Full node binary
└────────┬────────┘
         │
    ┌────┴────┬─────────────┐
    │         │             │
    ▼         ▼             ▼
┌───────┐ ┌───────┐ ┌───────────┐
│Network│ │  RPC  │ │ Consensus │
└───┬───┘ └───┬───┘ └─────┬─────┘
    │         │           │
    └────┬────┴───────────┘
         │
    ┌────┴────┐
    │ Storage │ ← RocksDB
    └────┬────┘
         │
    ┌────┴────┐
    │  Core   │ ← Blocks, Transactions, State
    └────┬────┘
         │
    ┌────┴────┐
    │ Crypto  │ ← Dilithium, Kyber, SHA3
    └─────────┘
```

---

## 💬 Community

- **GitHub Issues:** [Report bugs & request features](https://github.com/dill-lk/Lattice/issues)
- **Releases:** [Download latest](https://github.com/dill-lk/Lattice/releases)
- **Source Code:** [github.com/dill-lk/Lattice](https://github.com/dill-lk/Lattice)

---

## 📄 License

Dual-licensed under [MIT](LICENSE-MIT) or [Apache-2.0](LICENSE-APACHE) at your option.

---

<div align="center">

**Built with ❤️ for the post-quantum future**

⭐ Star this repo if you find it useful!

</div>

