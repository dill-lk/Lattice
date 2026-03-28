# ⛏️ Lattice Mining Guide

**Complete guide to mining LAT tokens on the Lattice blockchain**

---

## 🎯 Quick Start

### Prerequisites
- Lattice node running
- Wallet with address
- At least 2 CPU cores
- 4GB+ RAM

### Basic Mining Command
```bash
# Start mining with 4 threads
lattice-miner --threads 4

# Or with wallet address
lattice-miner --threads 4 --address <your-wallet-address>
```

---

## 📋 Step-by-Step Mining Setup

### Step 1: Install Lattice

Choose your installation method:

**Linux/macOS:**
```bash
curl -sSfL https://latticechain.io/install.sh | bash
```

**Windows:**
```powershell
irm https://latticechain.io/install.ps1 | iex
```

**From Source:**
```bash
git clone https://github.com/lattice-chain/lattice.git
cd lattice
make install
```

### Step 2: Create a Wallet

```bash
# Create new wallet
lattice-cli wallet create

# Example output:
# ✓ Created new wallet
#   Address: lat1qyg3d7xvzs8r39m2k7l5n8p6j4h2f9c3x5w8t0
#   Saved to: wallet.json
# 
# ⚠ IMPORTANT: Remember your password! It cannot be recovered.
```

**Save your wallet address!** This is where mining rewards will be sent.

### Step 3: Start a Node

**Option A: Development Mode (Single Node)**
```bash
# Quick start for testing
lattice-node --dev
```

**Option B: Production Mode**
```bash
# Start with configuration file
lattice-node --config ~/.lattice/config/node.toml
```

**Option C: Join Testnet**
```bash
# Connect to public testnet
lattice-node \
  --network testnet \
  --bootnodes /ip4/testnet.latticechain.io/tcp/30333/p2p/12D3KooW...
```

### Step 4: Wait for Sync

Check if your node is synced:

```bash
# Check sync status
lattice-cli node status

# Example output:
# Node Status:
#   Syncing: false
#   Current Height: 12345
#   Peers: 8
#   Status: Ready to mine ✓
```

⚠️ **Important:** Don't start mining until your node is fully synced!

### Step 5: Start Mining!

```bash
# Basic mining (auto-detect CPU cores)
lattice-miner

# Specify thread count
lattice-miner --threads 4

# With specific wallet address
lattice-miner --address lat1qyg3d7xvzs8r39m2k7l5n8p6j4h2f9c3x5w8t0

# With custom RPC endpoint
lattice-miner --rpc http://localhost:8545 --threads 4
```

---

## ⚙️ Mining Configuration

### Command-Line Options

```bash
lattice-miner [OPTIONS]

OPTIONS:
    --threads <N>         Number of mining threads (default: auto-detect)
    --address <ADDR>      Wallet address for rewards (default: from wallet.json)
    --rpc <URL>           RPC endpoint (default: http://127.0.0.1:8545)
    --difficulty <N>      Override difficulty (dev mode only)
    --verbose             Enable verbose logging
    --help                Show help message
```

### Configuration File

Create `~/.lattice/config/miner.toml`:

```toml
# Lattice Miner Configuration

# Number of CPU threads to use (0 = auto-detect)
threads = 4

# Wallet address for mining rewards
address = "lat1qyg3d7xvzs8r39m2k7l5n8p6j4h2f9c3x5w8t0"

# RPC endpoint of the node
rpc_url = "http://127.0.0.1:8545"

# Mining intensity (1-10, higher = more aggressive)
intensity = 7

# Log level (trace, debug, info, warn, error)
log_level = "info"
```

Run with config:
```bash
lattice-miner --config ~/.lattice/config/miner.toml
```

---

## 🚀 Optimization Tips

### 1. Choose Optimal Thread Count

```bash
# Check your CPU cores
nproc                    # Linux
sysctl -n hw.ncpu        # macOS
echo $NUMBER_OF_PROCESSORS  # Windows

# Rule of thumb:
# - Desktop mining: Use all cores
# - Server mining: Leave 1-2 cores for system
# - Laptop mining: Use 50-75% of cores (to prevent overheating)
```

**Examples:**
- **8-core CPU:** Use 6-8 threads
- **4-core CPU:** Use 3-4 threads
- **2-core CPU:** Use 2 threads

### 2. Optimize System Settings

**Linux:**
```bash
# Increase file descriptors
ulimit -n 65536

# Set CPU governor to performance
sudo cpupower frequency-set --governor performance

# Disable CPU throttling (if stable power)
echo performance | sudo tee /sys/devices/system/cpu/cpu*/cpufreq/scaling_governor
```

**Windows:**
```powershell
# Set power plan to High Performance
powercfg /setactive 8c5e7fda-e8bf-4a96-9a85-a6e23a8c635c

# Disable CPU throttling in BIOS
# (restart and enter BIOS setup)
```

### 3. Monitor Performance

```bash
# Check mining status
curl http://localhost:8545/metrics | grep mining

# Output:
# mining_attempts_total 1234567
# blocks_mined_total 42
# consensus_difficulty 1000000

# Check hashrate
lattice-cli mining stats
```

### 4. Cooling & Hardware

- ✅ Ensure good airflow
- ✅ Monitor CPU temperature (should stay below 80°C)
- ✅ Use thermal paste if needed
- ✅ Consider undervolting for efficiency
- ⚠️ Mining generates heat - plan accordingly!

---

## 💰 Mining Economics

### Block Rewards

```
Block Reward: 10 LAT
Block Time: ~15 seconds (target)
Daily Blocks: ~5,760 blocks
Daily Emission: ~57,600 LAT
```

### Estimated Earnings

| Hash Rate | Daily Blocks | Daily Earnings | Monthly Earnings |
|-----------|-------------|----------------|------------------|
| 1 hash/s  | ~0.1        | ~1 LAT         | ~30 LAT         |
| 10 hash/s | ~1          | ~10 LAT        | ~300 LAT        |
| 100 hash/s| ~10         | ~100 LAT       | ~3,000 LAT      |
| 1000 hash/s| ~100       | ~1,000 LAT     | ~30,000 LAT     |

*Note: Actual earnings depend on network difficulty and total network hashrate*

### Difficulty Adjustment

Lattice uses **dynamic difficulty adjustment**:
- Target block time: 15 seconds
- Adjustment period: Every 100 blocks
- Algorithm: Exponential moving average

If blocks are found too quickly → difficulty increases  
If blocks are found too slowly → difficulty decreases

---

## 🔍 Monitoring Your Mining

### Check Mining Status

```bash
# Via CLI
lattice-cli mining status

# Output:
# Mining Status:
#   Status: Mining
#   Hash Rate: 1234 H/s
#   Difficulty: 1000000
#   Last Block: 5 seconds ago
#   Blocks Mined: 42
#   Rewards Earned: 420 LAT
```

### View Mining Logs

```bash
# Follow logs in real-time
tail -f ~/.lattice/logs/miner.log

# Example log output:
# [INFO] Mining thread 1 started
# [INFO] Mining thread 2 started
# [INFO] Mining thread 3 started
# [INFO] Mining thread 4 started
# [INFO] Current hashrate: 1234 H/s
# [INFO] Mining attempt #12345...
# [SUCCESS] Block mined! Height: 100, Reward: 10 LAT
```

### Prometheus Metrics

```bash
# Get detailed metrics
curl http://localhost:9615/metrics | grep mining

# Key metrics:
# mining_attempts_total       - Total hash attempts
# blocks_mined_total          - Blocks successfully mined
# mining_hashrate             - Current hashrate
# mining_difficulty           - Current difficulty
# mining_rewards_earned_total - Total LAT earned
```

### Grafana Dashboard

If you're running the full monitoring stack:

```bash
# Start with monitoring
docker-compose up -d

# Access Grafana
open http://localhost:3000

# Default credentials:
# Username: admin
# Password: admin
```

---

## 🎮 Advanced Mining

### Mining Pool Setup

Create a mining pool configuration:

```toml
# pool.toml
[pool]
name = "My Mining Pool"
address = "lat1pool..."
min_payout = 10.0       # LAT
fee_percent = 2.0       # 2% pool fee

[server]
listen = "0.0.0.0:3333"
max_clients = 100

[strategy]
share_difficulty = 1000
vardiff_enabled = true
```

Run pool server:
```bash
lattice-pool --config pool.toml
```

Connect miners to pool:
```bash
lattice-miner --pool stratum+tcp://pool.example.com:3333 --username miner1
```

### Solo Mining vs Pool Mining

| Feature | Solo Mining | Pool Mining |
|---------|------------|-------------|
| Rewards | All (10 LAT) | Shared based on contribution |
| Frequency | Infrequent (luck-based) | Frequent (more stable) |
| Payout | When you find a block | Regular payouts |
| Variance | High | Low |
| Setup | Simple | Requires pool |

**Recommendation:**
- **Small miners (<100 H/s):** Join a pool for regular income
- **Large miners (>1000 H/s):** Solo mining can be profitable

### GPU Mining (Future)

*Note: Currently CPU-only. GPU mining coming in v0.2.0*

The Argon2id algorithm is memory-hard, making GPUs less efficient than for other blockchains. However, specialized GPU implementations are possible.

---

## 🐛 Troubleshooting

### Problem: "No blocks being mined"

**Solutions:**
```bash
# 1. Check if node is synced
lattice-cli node status

# 2. Check network difficulty
lattice-cli query difficulty

# 3. Increase threads
lattice-miner --threads 8

# 4. Check if other miners are on network
lattice-cli network peers
```

### Problem: "High CPU usage"

**Solutions:**
```bash
# Reduce thread count
lattice-miner --threads 2

# Set CPU affinity (Linux)
taskset -c 0,1 lattice-miner --threads 2

# Lower process priority
nice -n 19 lattice-miner --threads 4
```

### Problem: "Mining rewards not appearing"

**Check:**
```bash
# 1. Verify wallet address in miner
lattice-miner --address lat1your-address...

# 2. Check wallet balance
lattice-cli wallet balance

# 3. Check if blocks were orphaned
lattice-cli query block <block-number>

# 4. Wait for confirmations (6 blocks recommended)
```

### Problem: "RPC connection error"

**Solutions:**
```bash
# 1. Check if node is running
ps aux | grep lattice-node

# 2. Check RPC endpoint
curl http://localhost:8545/health

# 3. Specify correct RPC URL
lattice-miner --rpc http://localhost:8545

# 4. Check firewall
sudo ufw allow 8545
```

### Problem: "Out of memory"

**Solutions:**
```bash
# 1. Reduce threads
lattice-miner --threads 2

# 2. Increase swap space (Linux)
sudo fallocate -l 4G /swapfile
sudo mkswap /swapfile
sudo swapon /swapfile

# 3. Close other applications
# 4. Consider upgrading RAM
```

---

## 📊 Mining Statistics & Tools

### Calculate Your Potential Earnings

```bash
# Use the mining calculator
lattice-cli mining calculator \
  --hashrate 1000 \
  --power 100 \
  --electricity 0.10

# Output:
# Hash Rate: 1000 H/s
# Network Difficulty: 1000000
# Your Share: 0.1%
# 
# Estimated Earnings:
#   Daily: 100 LAT (~$10)
#   Weekly: 700 LAT (~$70)
#   Monthly: 3000 LAT (~$300)
# 
# Costs:
#   Power: 100W
#   Daily Cost: $0.24
#   Daily Profit: $9.76
# 
# ROI: Positive ✓
```

### Monitor Network Statistics

```bash
# Network hashrate
lattice-cli network hashrate

# Difficulty
lattice-cli network difficulty

# Average block time
lattice-cli network blocktime

# Your mining share
lattice-cli mining share
```

---

## 🎯 Mining Best Practices

### ✅ Do's

- ✅ Start with low thread count and increase gradually
- ✅ Monitor CPU temperature
- ✅ Keep your node synced
- ✅ Backup your wallet regularly
- ✅ Join the community for updates
- ✅ Use stable internet connection
- ✅ Run on dedicated hardware if possible

### ❌ Don'ts

- ❌ Don't mine on a laptop plugged in 24/7 (overheating)
- ❌ Don't mine on borrowed hardware without permission
- ❌ Don't run too many threads (leave resources for system)
- ❌ Don't skip wallet backups
- ❌ Don't mine on public WiFi (security risk)
- ❌ Don't expect instant profits (mining takes time)

---

## 🔐 Security Tips

### Secure Your Mining Setup

1. **Wallet Security**
   ```bash
   # Use strong password for wallet
   # Store wallet.json in secure location
   # Never share your private key
   chmod 600 ~/.lattice/wallet.json
   ```

2. **Network Security**
   ```bash
   # Enable firewall
   sudo ufw enable
   sudo ufw allow 30333/tcp  # P2P
   sudo ufw allow 8545/tcp   # RPC (only if needed)
   ```

3. **System Security**
   ```bash
   # Keep system updated
   sudo apt update && sudo apt upgrade -y
   
   # Run miner as non-root user
   sudo useradd -r -s /bin/false lattice-miner
   ```

---

## 📚 Additional Resources

### Documentation
- [Main README](../README.md)
- [Advanced Features](../ADVANCED_FEATURES.md)
- [API Documentation](https://docs.latticechain.io/api)

### Community
- **Discord:** https://discord.gg/lattice
- **Forum:** https://forum.latticechain.io
- **Mining Channel:** #mining on Discord

### Tools
- **Mining Calculator:** https://latticechain.io/calculator
- **Block Explorer:** https://explorer.latticechain.io
- **Pool List:** https://latticechain.io/pools

---

## 🎉 Happy Mining!

You're now ready to mine LAT tokens on the Lattice blockchain!

**Quick Reference:**
```bash
# 1. Create wallet
lattice-cli wallet create

# 2. Start node
lattice-node --dev

# 3. Start mining
lattice-miner --threads 4

# 4. Check balance
lattice-cli wallet balance
```

**Questions?** Join our [Discord](https://discord.gg/lattice) or check the [FAQ](https://docs.latticechain.io/faq).

---

**Good luck and happy mining! ⛏️💰**
