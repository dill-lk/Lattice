# 🛠️ Lattice Admin & Operator Guide

Official operations guide for the **unified `lattice` executable**.

> `lattice` is the primary interface.
> Legacy binaries such as `lattice-node`, `lattice-cli`, and `lattice-miner`
> now exist only as compatibility wrappers.

---

## Table of Contents

1. [Install](#1-install)
2. [Unified Command Model](#2-unified-command-model)
3. [Create a Wallet](#3-create-a-wallet)
4. [Run a Node](#4-run-a-node)
5. [Quick Mining Test](#5-quick-mining-test)
6. [Production Mining Setup](#6-production-mining-setup)
7. [Systemd Examples](#7-systemd-examples)
8. [Troubleshooting](#8-troubleshooting)

---

## 1. Install

### Linux / macOS

```bash
curl -fsSL https://raw.githubusercontent.com/dill-lk/Lattice/main/install.sh | bash
```

### Windows (PowerShell)

```powershell
Set-ExecutionPolicy -ExecutionPolicy RemoteSigned -Scope CurrentUser
irm https://raw.githubusercontent.com/dill-lk/Lattice/main/install.ps1 | iex
```

### Build from Source

```bash
git clone https://github.com/dill-lk/Lattice.git
cd Lattice
cargo build --release --bin lattice
sudo cp target/release/lattice /usr/local/bin/
```

---

## 2. Unified Command Model

Everything should be done through `lattice`:

```bash
lattice --help
lattice --version
lattice
lattice --node
lattice --mine 4
lattice --wallet-new
lattice --balance <ADDRESS>
lattice --send <ADDRESS> --amount 1.5

lattice wallet ...
lattice tx ...
lattice query ...
lattice contract ...
lattice status
lattice peers
lattice node ...
lattice miner ...
```

### Minimal Top-Level Paths

- `lattice` → quick local snapshot
- `lattice --node` → boot the full node daemon
- `lattice --mine 4` → start local mining using the default wallet as coinbase
- `lattice --wallet-new` → create `wallet.json`
- `lattice --balance <ADDRESS>` → query balance quickly
- `lattice --send <ADDRESS> --amount <LAT>` → quick transfer path

### Helpful UX shortcuts

- `lattice doctor` → diagnose wallet / RPC / data-dir setup
- `lattice miner ...` → if pointed at the default local RPC and no local node is reachable, it can fall back to integrated local miner-node behavior

### Advanced Paths

Use subcommands for fuller control:

```bash
lattice node --network devnet
lattice miner --coinbase <ADDRESS> --threads 8 --network devnet
lattice wallet create --output wallet.json
lattice tx send --to <ADDRESS> --amount 2.5 --wallet wallet.json
lattice query block latest
```

---

## 3. Create a Wallet

### Fast Path

```bash
lattice --wallet-new
```

This creates `wallet.json` in the current directory.

### Advanced Path

```bash
lattice wallet create --output ~/.lattice/wallet.json
lattice wallet address --wallet ~/.lattice/wallet.json
```

### Back It Up

```bash
cp wallet.json ~/wallet-backup-$(date +%Y%m%d).json
chmod 600 ~/wallet-backup-*.json
```

> ⚠️ If you lose both the keystore and password, funds are unrecoverable.

---

## 4. Run a Node

### Minimal Path

```bash
lattice --node
```

### Explicit Node Path

```bash
lattice node --network mainnet
lattice node --network testnet
lattice node --network devnet
```

### Useful Options

```bash
lattice node \
  --datadir ~/.lattice \
  --network devnet \
  --rpc-host 127.0.0.1 \
  --rpc-port 8545 \
  --p2p-port 30303 \
  --log-level info
```

### Write Default Config

```bash
lattice node --init
```

### Check Status

```bash
lattice status
lattice peers
```

---

## 5. Quick Mining Test

This is the fastest full local sanity check.

### Step 1 — Create wallet

```bash
lattice --wallet-new
lattice wallet address --wallet wallet.json
```

### Step 2 — Start a devnet node

```bash
lattice node --network devnet
```

### Step 3 — Mine using the top-level fast path

```bash
lattice --mine 4
```

### Step 4 — Check balance

```bash
lattice --balance wallet.json
```

---

## 6. Production Mining Setup

For real use, prefer separate node and miner processes.

### Start Node

```bash
lattice node \
  --datadir ~/.lattice \
  --network mainnet \
  --log-level info
```

### Start Miner

```bash
lattice miner \
  --coinbase <YOUR_ADDRESS> \
  --threads 8 \
  --rpc http://127.0.0.1:8545 \
  --network mainnet
```

### Sync Check Before Mining

```bash
lattice status
```

---

## 7. Systemd Examples

### Node Service

`/etc/systemd/system/lattice-node.service`

```ini
[Unit]
Description=Lattice Unified Node
After=network-online.target
Wants=network-online.target

[Service]
Type=simple
User=lattice
Group=lattice
WorkingDirectory=/var/lib/lattice
ExecStart=/usr/local/bin/lattice \
  node \
  --datadir /var/lib/lattice \
  --network mainnet \
  --rpc-host 127.0.0.1 \
  --rpc-port 8545 \
  --p2p-port 30303 \
  --log-level info
Restart=on-failure
RestartSec=10
LimitNOFILE=65536

PrivateTmp=true
NoNewPrivileges=true
ProtectSystem=strict
ReadWritePaths=/var/lib/lattice

[Install]
WantedBy=multi-user.target
```

### Miner Service

`/etc/systemd/system/lattice-miner.service`

```ini
[Unit]
Description=Lattice Unified Miner
After=lattice-node.service
Requires=lattice-node.service

[Service]
Type=simple
User=lattice
Group=lattice
WorkingDirectory=/var/lib/lattice
ExecStart=/usr/local/bin/lattice \
  miner \
  --coinbase YOUR_ADDRESS \
  --threads 6 \
  --rpc http://127.0.0.1:8545 \
  --network mainnet
Restart=on-failure
RestartSec=15

[Install]
WantedBy=multi-user.target
```

### Reload and Start

```bash
sudo systemctl daemon-reload
sudo systemctl enable --now lattice-node
sudo systemctl enable --now lattice-miner
sudo systemctl status lattice-node
sudo systemctl status lattice-miner
```

---

## 8. Troubleshooting

### Show local snapshot

```bash
lattice
```

### RPC not reachable

```bash
lattice status
curl -s http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"lat_blockNumber","params":[],"id":1}'
```

### Default wallet missing for `--mine`

```bash
lattice --wallet-new
lattice --mine 4
```

### Invalid coinbase address

```bash
lattice wallet address --wallet wallet.json
```

Use that address for:

```bash
lattice miner --coinbase <ADDRESS>
```

### Clean local dev data

```bash
rm -rf ~/.lattice
```

---

## Security Notes

- keep RPC private unless you intentionally expose it
- run services as a non-root user
- keep wallet files permission-locked
- back up keystores offline
- prefer `lattice` over legacy wrappers in scripts and automation

---

## Further Reading

| Document | Purpose |
|---|---|
| [README.md](README.md) | quick-start overview |
| [MINING_GUIDE.md](MINING_GUIDE.md) | mining setup and tuning |
| [DEPLOYMENT.md](DEPLOYMENT.md) | deployment patterns |
| [docs/protocol-baseline.md](docs/protocol-baseline.md) | canonical behavior baseline |
