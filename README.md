# ⛏️ Lattice — Quantum-Resistant Blockchain

**CPU-friendly mining · Post-quantum cryptography · Open source**

Lattice is a quantum-resistant blockchain secured by CRYSTALS-Dilithium3 signatures and an Argon2-based memory-hard Proof-of-Work algorithm. Anyone with a regular CPU can participate.

---

## 🚀 Install in 60 Seconds

Binaries are published on the [GitHub Releases](https://github.com/dill-lk/Lattice/releases) page.
The installer downloads the latest release automatically.

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/dill-lk/Lattice/main/install.sh | bash
```

> To choose a custom install directory: `bash install.sh --dir /usr/local/bin`
> To uninstall: `bash install.sh --uninstall`

### Windows (PowerShell)

```powershell
# Run once if needed (allows local script execution):
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

irm https://raw.githubusercontent.com/dill-lk/Lattice/main/install.ps1 | iex
```

> Custom directory: `.\install.ps1 -InstallDir "C:\Lattice"`
> Uninstall: `.\install.ps1 -Uninstall`

The installer places three binaries on your `PATH`:

| Binary | Purpose |
|--------|---------|
| `lattice-node` | Full blockchain node + built-in miner |
| `lattice-cli` | Wallet management & RPC queries |
| `lattice-miner` | Standalone multi-threaded miner |

---

## ⚙️ Setup: Three Steps

### 1 — Create a Wallet

```bash
lattice-cli wallet create
# Saved to ./wallet.json by default
# Note the address printed — you'll use it as your coinbase
```

### 2 — Start the Node

```bash
# Mainnet (default)
lattice-node

# With mining enabled right away (replace with your address)
lattice-node --mine --mining-threads 4 --coinbase <your-address>
```

The node starts the RPC server on `127.0.0.1:8545` and the P2P listener on `0.0.0.0:30303`.

### 3 — Check Node Status

```bash
lattice-cli node status
```

Wait until the node reports it is fully synced before doing anything else.

---

## ⛏️ Mining

### Option A — Built-in miner (simplest)

Pass `--mine` directly to `lattice-node`:

```bash
lattice-node \
  --mine \
  --mining-threads 4 \
  --coinbase <your-address>
```

### Option B — Standalone miner (recommended for performance)

Run the node and the miner in separate terminals:

```bash
# Terminal 1 — node
lattice-node

# Terminal 2 — miner (connects to node via RPC)
lattice-miner --threads 4 --coinbase <your-address>
```

### Mining thread recommendation

| CPU cores | Recommended `--threads` |
|-----------|------------------------|
| 2 | 2 |
| 4 | 3–4 |
| 8 | 6–8 |
| 16+ | 12–16 |

---

## 💵 Block Rewards

| Parameter | Value |
|-----------|-------|
| Reward per block | 10 LAT |
| Target block time | ~15 seconds |
| Daily emission | ~57,600 LAT |

---

## 📖 Wallet Commands

```bash
# Create new wallet
lattice-cli wallet create

# Show your address
lattice-cli wallet address

# Check balance (requires node running)
lattice-cli wallet balance <address>

# Export private key
lattice-cli wallet export

# Send tokens
lattice-cli tx send --to <address> --amount 1.5
```

---

## ⚙️ System Requirements

| Tier | CPU | RAM | Disk |
|------|-----|-----|------|
| Minimum | 2 cores | 4 GB | 20 GB |
| Recommended | 4+ cores | 8 GB | 50 GB SSD |
| Optimal | 8+ cores | 16 GB | 100 GB SSD |

---

## 🔐 Wallet Backup

Your wallet is at `./wallet.json` (or the path you specified at creation).

```bash
# Backup (Linux/macOS)
cp wallet.json ~/wallet-backup-$(date +%Y%m%d).json
chmod 600 ~/wallet-backup-*.json
```

**⚠️ If you lose your wallet file and password, your LAT is gone forever. Back it up.**

---

## 🆘 Troubleshooting

| Problem | Fix |
|---------|-----|
| Node won't start | Check port 30303 isn't already in use |
| Miner shows RPC error | Make sure `lattice-node` is running first |
| Invalid coinbase address | Address must be from `lattice-cli wallet address`, not a Bitcoin address |
| No peers | Open port 30303 TCP in your firewall |
| Low hashrate | Increase `--threads`, close background apps |

```bash
# Diagnose connectivity
lattice-cli node peers

# Open P2P port (Ubuntu)
sudo ufw allow 30303/tcp
```

---

## 📚 Documentation

| Document | Contents |
|----------|---------|
| [TOKENOMICS.md](TOKENOMICS.md) | Token supply, genesis allocation, vesting schedule |
| [ADMIN.md](ADMIN.md) | Full operator guide — hosting a node, systemd service, advanced config |
| [MINING_GUIDE.md](MINING_GUIDE.md) | In-depth mining optimisation |
| [DEPLOYMENT.md](DEPLOYMENT.md) | Docker & production deployment |
| [CONTRIBUTING.md](CONTRIBUTING.md) | How to build from source and contribute |
| [docs/api-reference.md](docs/api-reference.md) | JSON-RPC API reference |

---

## 📝 For Developers

```bash
# Build from source (requires Rust 1.75+ and RocksDB dev libraries)
git clone https://github.com/dill-lk/Lattice.git
cd Lattice
cargo build --release

# Run tests
cargo test

# Lint
cargo clippy --all -- -D warnings
```

See [CONTRIBUTING.md](CONTRIBUTING.md) for the full development setup guide.

---

## 💬 Community & Support

- **GitHub Issues:** https://github.com/dill-lk/Lattice/issues
- **Releases:** https://github.com/dill-lk/Lattice/releases
- **Source:** https://github.com/dill-lk/Lattice

---

## 📄 License

MIT OR Apache-2.0

