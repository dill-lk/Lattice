# ⛏️ Lattice Mining Guide

Unified mining guide for the **official `lattice` executable**.

> Preferred commands:
> - `lattice --mine 4` for a fast local path
> - `lattice miner ...` for the full standalone miner workflow

---

## 1. Quickest Local Path

```bash
# create default wallet
lattice --wallet-new

# start a devnet node in one terminal
lattice node --network devnet

# start local mining in another terminal
lattice --mine 4
```

Check balance:

```bash
lattice --balance wallet.json
```

---

## 2. Production-Style Mining

For more serious use, keep node and miner separate.

### Step 1 — Create wallet

```bash
lattice wallet create --output wallet.json
lattice wallet address --wallet wallet.json
```

### Step 2 — Start the node

```bash
lattice node \
  --network mainnet \
  --datadir ~/.lattice \
  --log-level info
```

### Step 3 — Verify sync

```bash
lattice status
```

### Step 4 — Start standalone miner

```bash
lattice miner \
  --coinbase <YOUR_ADDRESS> \
  --threads 8 \
  --rpc http://127.0.0.1:8545 \
  --network mainnet
```

---

## 3. Miner Modes

### Minimal local mode

```bash
lattice --mine 4
```

Uses:
- default wallet file: `wallet.json`
- local integrated node path
- default RPC endpoint

### Standalone miner with auto-node fallback

```bash
lattice miner --coinbase <ADDRESS> --threads 4 --network mainnet
```

If the configured RPC endpoint is the default local endpoint and no node is reachable,
Lattice will automatically switch into an integrated local miner-node path so the
user is not blocked by missing setup.

### Explicit standalone mode

```bash
lattice miner --coinbase <ADDRESS> --threads 4 --network devnet
```

### Advanced standalone mode

```bash
lattice miner \
  --coinbase <ADDRESS> \
  --threads 8 \
  --rpc http://127.0.0.1:8545 \
  --poll-interval 1000 \
  --stats-interval 10 \
  --network mainnet
```

---

## 4. Thread Recommendations

| CPU cores | Recommended threads |
|---|---:|
| 2 | 2 |
| 4 | 3–4 |
| 8 | 6–8 |
| 16 | 12–16 |
| 32+ | leave some headroom for the OS and node |

---

## 5. Memory-Hard PoW Profiles

Lattice uses Argon2id-based PoW.

Current built-in profiles:

| Network | Memory Cost | Intended Use |
|---|---:|---|
| Devnet | 512 KiB | local speed / development |
| Testnet | 4 MiB | lighter public testing |
| Mainnet | 64 MiB | full security profile |

---

## 6. What to Watch While Mining

### Use the default snapshot

```bash
lattice
```

### Check status

```bash
lattice status
lattice peers
```

### Check balance

```bash
lattice --balance <ADDRESS>
# or
lattice --balance wallet.json
```

### Inspect chain data

```bash
lattice query block latest
lattice query account <ADDRESS>
```

---

## 7. Operator Tips

### Good practices

- mine only after the node is synced
- keep wallet backups offline
- leave some CPU for the OS and the node
- monitor heat on laptops and desktops
- use `lattice miner ...` instead of old compatibility wrappers

### Don’t do this

- don’t assume the node is synced without checking `lattice status`
- don’t expose RPC publicly without understanding the risk
- don’t rely on old docs that still say `lattice-miner` as the primary interface

---

## 8. Troubleshooting

### `lattice --mine 4` fails because wallet.json is missing

```bash
lattice --wallet-new
```

### RPC connection errors

```bash
lattice status
curl -s http://127.0.0.1:8545 \
  -H "Content-Type: application/json" \
  -d '{"jsonrpc":"2.0","method":"lat_blockNumber","params":[],"id":1}'
```

### Invalid coinbase

```bash
lattice wallet address --wallet wallet.json
```

Use that exact address in:

```bash
lattice miner --coinbase <ADDRESS>
```

### Low hashrate

- increase threads carefully
- reduce background CPU load
- check thermal throttling
- try devnet first to confirm your setup works

---

## 9. Pool-Ready Direction

Lattice does not yet ship a finished pool protocol, but the current CLI and miner
layout is being shaped so a future pool-oriented flow can sit naturally on top of:

- `lattice miner ...`
- explicit RPC endpoints
- structured JSON output
- clearer diagnostics and benchmarking

That means pool integration can be added later without redesigning the whole operator UX.

## 10. Compatibility Note

Legacy binaries still exist as wrappers:
- `lattice-node`
- `lattice-cli`
- `lattice-miner`

But the official interface is now:

```bash
lattice ...
```

---

## 10. Recommended Reality-Based Workflow

1. `lattice --wallet-new`
2. `lattice node --network devnet`
3. `lattice --mine 4`
4. `lattice --balance wallet.json`
5. `lattice query block latest`

That is the cleanest current path for validating mining end-to-end.
