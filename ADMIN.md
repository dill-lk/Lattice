# 🛠️ Lattice Admin & Operator Guide

Everything you need to install, configure, run, and mine on the Lattice blockchain —
whether you're running a personal node, a mining rig, or a public-facing server.

---

## Table of Contents

1. [Install the Binaries](#1-install-the-binaries)
2. [Binary Reference](#2-binary-reference)
3. [Create a Wallet](#3-create-a-wallet)
4. [Run a Node](#4-run-a-node)
5. [Test Mining (Quick Start)](#5-test-mining-quick-start)
6. [Production Mining Setup](#6-production-mining-setup)
7. [Hosting a Public Node](#7-hosting-a-public-node)
8. [Configuration File Reference](#8-configuration-file-reference)
9. [CLI Reference](#9-cli-reference)
10. [Troubleshooting](#10-troubleshooting)

---

## 1. Install the Binaries

All releases are published at **https://github.com/dill-lk/Lattice/releases**.
The installer fetches the latest release automatically.

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/dill-lk/Lattice/main/install.sh | bash
```

**Options:**

```bash
# Install to a specific directory
bash install.sh --dir /usr/local/bin

# Uninstall
bash install.sh --uninstall

# Show help
bash install.sh --help
```

> The installer adds the binary directory to your shell's `PATH` automatically.
> Open a new terminal (or run `source ~/.bashrc`) before continuing.

### Windows (PowerShell)

```powershell
# Allow local script execution (run once, as your user):
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser

# Install
irm https://raw.githubusercontent.com/dill-lk/Lattice/main/install.ps1 | iex
```

**Options:**

```powershell
# Install to a specific directory
.\install.ps1 -InstallDir "C:\Lattice\bin"

# Uninstall
.\install.ps1 -Uninstall
```

> After installation, restart your terminal so the updated `PATH` takes effect.

### Manual Download

1. Go to https://github.com/dill-lk/Lattice/releases/latest
2. Download the archive for your platform:
   - `lattice-linux-amd64.tar.gz`
   - `lattice-macos-amd64.tar.gz`
   - `lattice-windows-amd64.zip`
3. Extract and move the three binaries (`lattice-node`, `lattice-cli`, `lattice-miner`) to any directory on your `PATH`.

### Build from Source

Requires **Rust 1.75+** and RocksDB dev libraries.

```bash
# Install RocksDB dependencies
# Ubuntu/Debian:
sudo apt install -y build-essential clang librocksdb-dev pkg-config
# macOS:
brew install rocksdb

# Install Rust
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Build
git clone https://github.com/dill-lk/Lattice.git
cd Lattice
cargo build --release

# Binaries land in target/release/
sudo cp target/release/lattice-{node,cli,miner} /usr/local/bin/
```

---

## 2. Binary Reference

| Binary | Purpose | Default ports |
|--------|---------|---------------|
| `lattice-node` | Full node: syncs the chain, serves the RPC API, optionally mines | P2P `30303`, RPC `8545` |
| `lattice-cli` | Wallet, queries, transactions — talks to a running node via RPC | — |
| `lattice-miner` | Standalone CPU miner — fetches work from the node over RPC | — |

Verify installation:

```bash
lattice-node   --version
lattice-cli    --version
lattice-miner  --version
```

---

## 3. Create a Wallet

A wallet must be created before mining (you need an address as your coinbase).

```bash
# Create a new wallet (saves to wallet.json by default)
lattice-cli wallet create

# Save to a specific path
lattice-cli wallet create --output ~/.lattice/wallet.json

# Show your address
lattice-cli wallet address

# Show your address from a specific wallet file
lattice-cli wallet address --wallet ~/.lattice/wallet.json
```

Example output:

```
✓ Created new wallet
  Address: 1Q3rx3W6v3ctE6bNrtGimCew2Fx4P1frQL
  Saved to: wallet.json

⚠ Remember your password — it cannot be recovered.
```

**Back up your wallet immediately:**

```bash
# Linux/macOS
cp wallet.json ~/wallet-backup-$(date +%Y%m%d).json
chmod 600 ~/wallet-backup-*.json

# Windows PowerShell
Copy-Item wallet.json "$env:USERPROFILE\wallet-backup-$(Get-Date -f yyyyMMdd).json"
```

> ⚠️ The wallet file contains your encrypted private key. If you lose both the file
> and your password, your LAT balance is permanently inaccessible.

---

## 4. Run a Node

### 4.1 Quick Start (no config file)

```bash
# Mainnet — run with all defaults
lattice-node

# Specify data directory
lattice-node --datadir ~/.lattice/data

# Connect to testnet instead
lattice-node --network testnet

# Verbose logging
lattice-node --log-level debug
```

### 4.2 All Node Flags

```
lattice-node [OPTIONS]

Options:
  -c, --config <FILE>               Path to TOML configuration file
      --datadir <DIR>               Data directory (default: OS-specific, e.g. ~/.local/share/lattice)
      --network <NAME>              mainnet | testnet | devnet  (default: mainnet)
      --rpc-host <HOST>             RPC listen host (default: 127.0.0.1)
      --rpc-port <PORT>             RPC listen port (default: 8545)
      --no-rpc                      Disable RPC server
      --p2p-port <PORT>             P2P listen port (default: 30303)
      --mine                        Enable built-in miner
      --mining-threads <N>          Threads for built-in miner (default: CPU count)
      --coinbase <ADDR>             Reward address for built-in miner
      --log-level <LEVEL>           error | warn | info | debug | trace (default: info)
      --bootnodes <LIST>            Comma-separated multiaddrs for peer discovery
      --init                        Write a default config file and exit
  -h, --help                        Print help
  -V, --version                     Print version
```

### 4.3 Generate a Config File

```bash
# Write defaults to node.toml and exit
lattice-node --init
# Then edit node.toml and start with:
lattice-node --config node.toml
```

### 4.4 Check Node Status

```bash
# Via CLI (node must be running)
lattice-cli node status

# Via raw RPC
curl -s http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"lat_blockNumber","params":[],"id":1}'
```

---

## 5. Test Mining (Quick Start)

This is the fastest way to verify that mining works on your machine using a local
single-node network.

### Step 1 — Create a wallet

```bash
lattice-cli wallet create --output wallet.json
MY_ADDRESS=$(lattice-cli wallet address --wallet wallet.json)
echo "Coinbase: $MY_ADDRESS"
```

### Step 2 — Start the node with mining enabled

```bash
lattice-node \
  --network devnet \
  --datadir /tmp/lattice-test \
  --mine \
  --mining-threads 2 \
  --coinbase "$MY_ADDRESS" \
  --log-level info
```

You should see log lines like:

```
INFO lattice_node: Mining: enabled (2 threads)
INFO lattice_node: Coinbase: 1Q3rx3W6v3ctE6bNrtGimCew2Fx4P1frQL
INFO lattice_node: Node started successfully
```

### Step 3 — Verify blocks are being mined

In a second terminal:

```bash
# Block height should increase
lattice-cli node status

# Check your balance (takes a few blocks to accumulate)
lattice-cli wallet balance "$MY_ADDRESS"
```

### Step 4 — Stop and clean up

Press `Ctrl+C` in the node terminal, then:

```bash
rm -rf /tmp/lattice-test
```

---

## 6. Production Mining Setup

For serious mining, run the node and the standalone miner in separate processes.
This allows the miner to restart without interrupting the node, and makes it easier
to scale threads independently.

### Architecture

```
┌───────────────┐   JSON-RPC (port 8545)   ┌─────────────────────┐
│ lattice-node  │ ◄────────────────────── │ lattice-miner        │
│ (syncs chain) │                          │ (finds PoW nonces)   │
└───────────────┘                          └─────────────────────┘
```

### Step 1 — Start the node (no `--mine` flag)

```bash
lattice-node --datadir ~/.lattice/data --log-level info
```

Wait until it is fully synced:

```bash
lattice-cli node status
# Syncing: false  ← wait for this
```

### Step 2 — Start the standalone miner

```bash
# Basic
lattice-miner --coinbase <your-address> --threads 4

# All options
lattice-miner \
  --coinbase <your-address> \
  --threads 8 \
  --rpc http://127.0.0.1:8545 \
  --poll-interval 1000 \
  --stats-interval 10
```

### Miner Flags

```
lattice-miner [OPTIONS]

Options:
  -c, --coinbase <ADDR>          Reward address (required)
  -t, --threads <N>              CPU threads (default: 0 = auto-detect)
  -r, --rpc <URL>                Node RPC URL (default: http://127.0.0.1:8545)
      --poll-interval <MS>       How often to check for new work in ms (default: 1000)
      --stats-interval <SECS>    How often to print stats in seconds (default: 10)
  -h, --help                     Print help
  -V, --version                  Print version
```

### Thread Recommendations

| CPU cores | `--threads` |
|-----------|------------|
| 2 | 2 |
| 4 | 3–4 |
| 8 | 6–8 |
| 16 | 12–16 |
| Server (32+) | 24–28 |

Leave at least 1–2 cores free for the OS and node.

---

## 7. Hosting a Public Node

### 7.1 Firewall

Open the P2P port so other nodes can reach you:

```bash
# Ubuntu/Debian (ufw)
sudo ufw allow 30303/tcp comment "Lattice P2P"
sudo ufw reload

# Keep RPC private — only expose it via SSH tunnel or reverse proxy
# Do NOT open port 8545 to the internet unless you add authentication.
```

### 7.2 Dedicated System User

```bash
sudo useradd --system --no-create-home --shell /usr/sbin/nologin lattice
sudo mkdir -p /var/lib/lattice
sudo chown lattice:lattice /var/lib/lattice
sudo cp $(which lattice-node) /opt/lattice/lattice-node
```

### 7.3 Systemd Service — Node

Create `/etc/systemd/system/lattice-node.service`:

```ini
[Unit]
Description=Lattice Blockchain Node
Documentation=https://github.com/dill-lk/Lattice
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=lattice
Group=lattice
WorkingDirectory=/var/lib/lattice
ExecStart=/opt/lattice/lattice-node \
    --datadir /var/lib/lattice \
    --network mainnet \
    --rpc-host 127.0.0.1 \
    --rpc-port 8545 \
    --p2p-port 30303 \
    --log-level info
Restart=on-failure
RestartSec=10
LimitNOFILE=65536

# Harden the service
PrivateTmp=true
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=/var/lib/lattice

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now lattice-node
sudo systemctl status lattice-node

# Follow logs
sudo journalctl -u lattice-node -f
```

### 7.4 Systemd Service — Standalone Miner (optional)

Create `/etc/systemd/system/lattice-miner.service`:

```ini
[Unit]
Description=Lattice Blockchain Miner
After=lattice-node.service
Requires=lattice-node.service

[Service]
Type=simple
User=lattice
Group=lattice
WorkingDirectory=/var/lib/lattice
# Replace the coinbase address with your own
ExecStart=/opt/lattice/lattice-miner \
    --coinbase 1YourAddressHere \
    --threads 6 \
    --rpc http://127.0.0.1:8545
Restart=on-failure
RestartSec=15

[Install]
WantedBy=multi-user.target
```

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now lattice-miner
sudo systemctl status lattice-miner
```

### 7.5 Mining Inside the Node (alternative for small servers)

If you'd rather keep everything in one process:

```ini
ExecStart=/opt/lattice/lattice-node \
    --datadir /var/lib/lattice \
    --network mainnet \
    --mine \
    --mining-threads 4 \
    --coinbase 1YourAddressHere \
    --log-level info
```

### 7.6 Log Rotation

Create `/etc/logrotate.d/lattice`:

```
/var/log/lattice/*.log {
    daily
    rotate 14
    compress
    delaycompress
    notifempty
    missingok
    create 0640 lattice lattice
    sharedscripts
    postrotate
        systemctl reload lattice-node 2>/dev/null || true
    endscript
}
```

### 7.7 Database Backup

```bash
#!/usr/bin/env bash
# /opt/lattice/backup.sh — run via cron e.g. "0 3 * * * /opt/lattice/backup.sh"
set -euo pipefail
BACKUP_DIR="/backup/lattice/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

systemctl stop lattice-node
cp -r /var/lib/lattice "$BACKUP_DIR/"
systemctl start lattice-node

echo "Backup saved to $BACKUP_DIR"
```

---

## 8. Configuration File Reference

Generate a default config file:

```bash
lattice-node --init
# Writes node.toml to the current directory
```

Full annotated example:

```toml
# Lattice Node Configuration

[network]
# Which network to join: mainnet | testnet | devnet
name = "mainnet"

[p2p]
# Address and port this node listens on for peers
listen_addr = "0.0.0.0:30303"

# Peers to connect to on startup (comma-separated multiaddrs)
# Leave empty to rely only on peer discovery
bootstrap_nodes = []

# Maximum simultaneous peer connections
max_peers = 50

[rpc]
# Bind address for the JSON-RPC server
# Set to "0.0.0.0:8545" to expose publicly (add a firewall rule first!)
listen_addr = "127.0.0.1:8545"

enabled = true

[mining]
# Set enabled = true to turn on the built-in miner
enabled = false

# Number of CPU threads (0 = auto-detect)
threads = 0

# Coinbase address — mining rewards go here
coinbase = ""

[storage]
# Path to the RocksDB database directory
db_path = "/var/lib/lattice"

# In-memory block cache size in MB
cache_size = 256

[logging]
# Verbosity: error | warn | info | debug | trace
level = "info"
```

---

## 9. CLI Reference

### Wallet

```bash
lattice-cli wallet create [--output <FILE>]
lattice-cli wallet address [--wallet <FILE>]
lattice-cli wallet balance <ADDRESS>
lattice-cli wallet export [--wallet <FILE>]
lattice-cli wallet import keystore <FILE> [--output <FILE>]
lattice-cli wallet import private-key <HEX> [--output <FILE>]
```

### Transactions

```bash
# Send LAT
lattice-cli tx send \
  --to <ADDRESS> \
  --amount 1.5 \          # in LAT (decimal)
  --wallet wallet.json

# Send exact amount in wei
lattice-cli tx send --to <ADDRESS> --amount 1500000000000000000 --wei

# Check transaction status
lattice-cli tx status <TX-HASH>

# Decode a raw transaction
lattice-cli tx decode <HEX>
```

### Queries

```bash
# Block by number
lattice-cli query block 1000
lattice-cli query block latest
lattice-cli query block latest --include-txs

# Block by hash
lattice-cli query block 0xabcd...

# Transaction by hash
lattice-cli query tx 0xabcd...

# Account info
lattice-cli query account <ADDRESS>
```

### Node

```bash
lattice-cli node status    # Sync status, height, peer count
lattice-cli node peers     # List connected peers
```

### Global Options

```bash
# All sub-commands accept --rpc to point at a non-default node
lattice-cli --rpc http://192.168.1.10:8545 node status
```

---

## 10. Troubleshooting

### Node won't start

```bash
# Check for another process on port 30303 or 8545
ss -tlnp | grep -E '30303|8545'

# Check logs
sudo journalctl -u lattice-node -n 50 --no-pager

# Test with a fresh data directory
lattice-node --datadir /tmp/lattice-debug --network devnet --log-level debug
```

### "Invalid coinbase address" error

The coinbase address must be a **Lattice address** (Base58Check with SHA3 checksum),
not a Bitcoin or Ethereum address.

```bash
# Generate a valid address
lattice-cli wallet create
lattice-cli wallet address
```

Use the address printed by `lattice-cli wallet address` as your `--coinbase` value.

### Miner shows RPC error

```bash
# 1. Is the node running?
pgrep -a lattice-node

# 2. Is the RPC server listening?
curl -s http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"lat_blockNumber","params":[],"id":1}'

# 3. Is the node fully synced?
lattice-cli node status

# 4. Specify the URL explicitly
lattice-miner --rpc http://127.0.0.1:8545 --coinbase <addr> --threads 2
```

### No peers / isolated node

```bash
# List peers
lattice-cli node peers

# Check firewall — port 30303 TCP must be reachable from outside
sudo ufw status
curl -s https://api.ipify.org   # your public IP — check it's reachable

# Specify bootnodes manually
lattice-node --bootnodes "/ip4/1.2.3.4/tcp/30303/p2p/12D3KooW..."
```

### Low hashrate

- Increase `--threads` to match your CPU count (leave 1–2 for the OS).
- Close background applications to free CPU.
- Check CPU temperature — thermal throttling silently reduces performance. Target below 80 °C.
- On Linux, set the CPU governor: `echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor`

### High memory usage

The Argon2 PoW algorithm is memory-hard by design. Each mining thread uses ~64 MB.

```
Memory needed ≈ threads × 64 MB
```

Reduce `--threads` if you run out of RAM.

### Wallet password forgotten

There is no password recovery. The private key is encrypted with the password using Argon2.
If you still have the wallet file, the only option is to brute-force your own password.
If you have lost both the wallet file and the password, the funds are unrecoverable.

**Always keep an offline backup of `wallet.json` in a safe location.**

---

## Security Notes

- **Never expose port 8545 (RPC) to the internet** without authentication. Anyone who can reach
  the RPC endpoint can read chain data and submit transactions from unlocked wallets.
- Run the node as a non-root, non-login system user (`lattice`).
- Store wallet files with permissions `600` (owner-read-only).
- Keep the binary up to date — check https://github.com/dill-lk/Lattice/releases regularly.

---

## Further Reading

| Document | Contents |
|----------|---------|
| [README.md](README.md) | Quick-start overview |
| [MINING_GUIDE.md](MINING_GUIDE.md) | Mining optimisation deep-dive |
| [DEPLOYMENT.md](DEPLOYMENT.md) | Docker & multi-node deployment |
| [CONTRIBUTING.md](CONTRIBUTING.md) | Build from source, coding conventions |
| [docs/api-reference.md](docs/api-reference.md) | Full JSON-RPC API reference |
