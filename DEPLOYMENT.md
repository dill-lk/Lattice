# Lattice Deployment Guide

Deployment guide for the **unified `lattice` executable**.

> Official runtime entrypoint: `lattice`
> 
> Use:
> - `lattice node ...` for full-node operation
> - `lattice miner ...` for standalone mining
> - `lattice` for quick local status snapshot

---

## 1. Build

```bash
git clone https://github.com/dill-lk/Lattice.git
cd Lattice
cargo build --release --bin lattice
ls -lh target/release/lattice
```

---

## 2. Local Deployment Patterns

### Snapshot

```bash
lattice
```

### Run a node

```bash
lattice node --network mainnet
```

### Run a miner against a node

```bash
lattice miner \
  --coinbase <ADDRESS> \
  --threads 8 \
  --rpc http://127.0.0.1:8545 \
  --network mainnet
```

### Quick one-command local mining

```bash
lattice --wallet-new
lattice --node
# in another shell
lattice --mine 4
```

---

## 3. Example Mainnet Node Command

```bash
lattice node \
  --datadir /var/lib/lattice \
  --network mainnet \
  --rpc-host 127.0.0.1 \
  --rpc-port 8545 \
  --p2p-port 30303 \
  --log-level info
```

---

## 4. Example Testnet / Devnet

### Testnet

```bash
lattice node --network testnet --datadir /var/lib/lattice-testnet
```

### Devnet

```bash
lattice node --network devnet --datadir /tmp/lattice-devnet
```

---

## 5. Systemd Services

### Node

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

### Miner

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

---

## 6. Docker

### Build image

```bash
docker build -t lattice:latest .
```

### Run container

```bash
docker run -d \
  --name lattice-mainnet \
  -p 30303:30303 \
  -p 8545:8545 \
  -v /var/lib/lattice:/data \
  lattice:latest \
  node --datadir /data --network mainnet
```

### Container defaults

The Dockerfile entrypoint is:

```text
lattice
```

The default command is:

```text
--node
```

So plain `docker run lattice:latest` will boot the unified node path.

---

## 7. Firewall / Security

### UFW example

```bash
sudo ufw allow 30303/tcp
# keep 8545 private unless intentionally exposing RPC
```

### RPC recommendation

Keep RPC bound locally when possible:

```bash
lattice node --rpc-host 127.0.0.1 --rpc-port 8545
```

---

## 8. Health Checks

### Quick operator check

```bash
lattice
lattice status
lattice peers
```

### Raw RPC check

```bash
curl -s http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"lat_blockNumber","params":[],"id":1}'
```

---

## 9. Backup Pattern

```bash
#!/usr/bin/env bash
set -euo pipefail

BACKUP_DIR="/backup/lattice/$(date +%Y%m%d-%H%M%S)"
mkdir -p "$BACKUP_DIR"

systemctl stop lattice-node
cp -r /var/lib/lattice "$BACKUP_DIR/"
systemctl start lattice-node

echo "Backup saved to $BACKUP_DIR"
```

---

## 10. Notes on Current Reality

Lattice currently has:
- a real unified CLI and node/miner flows
- improved protocol correctness in the official path
- compatibility wrappers for old binary names

Lattice does **not** yet claim in this guide that:
- networking is fully production-hardened
- public mainnet launch is complete
- all advanced explorer / admin / pool features are finished

This guide is intentionally aligned with the current code and project status.

---

## 11. Recommended Launch Order

1. local devnet
2. local multi-node testnet
3. small public testnet
4. long-running stability period
5. mainnet readiness review

---

## Further Reading

- [README.md](README.md)
- [ADMIN.md](ADMIN.md)
- [MINING_GUIDE.md](MINING_GUIDE.md)
- [docs/protocol-baseline.md](docs/protocol-baseline.md)
