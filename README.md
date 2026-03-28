# ⛏️ Lattice - Mine Quantum-Resistant Cryptocurrency
<img width="300" height="300" alt="lattice" src="https://github.com/user-attachments/assets/a28ce394-f794-408c-bf6f-d7d6cfdcd6b6" />

**Start earning LAT tokens today! CPU-friendly mining, no expensive GPUs needed.**

Lattice is the world's first production-ready quantum-resistant blockchain. Mine with your regular computer and earn rewards while securing the future of cryptocurrency.

---

## 💰 Why Mine Lattice?

| Feature | Benefit |
|---------|---------|
| **🖥️ CPU-Friendly** | Mine with any computer - no expensive GPUs or ASICs needed |
| **💎 Fair Distribution** | ASIC-resistant algorithm ensures fair mining for everyone |
| **🔐 Future-Proof** | Quantum-resistant technology protects your earnings forever |
| **⚡ Fast Blocks** | ~15 second block time = frequent rewards |
| **💵 Predictable Rewards** | 10 LAT per block, ~57,600 LAT daily emission |

---

## 🚀 Quick Start - Start Mining in 60 Seconds

### **Linux / macOS**
```bash
curl -sSfL https://latticechain.io/install.sh | bash
./start-mining.sh
```

### **Windows (PowerShell)**
```powershell
irm https://latticechain.io/install.ps1 | iex
.\start-mining.ps1
```

**That's it!** 🎉 You're now mining LAT tokens! Rewards will automatically go to your wallet.

---

## 💵 How Much Can I Earn?

**Mining Rewards:** 10 LAT per block (every ~15 seconds)

### Estimated Daily Earnings

| Your Hardware | Hashrate | Daily Earnings | Monthly Earnings |
|---------------|----------|----------------|------------------|
| 4-core CPU (Basic) | ~100 H/s | ~10 LAT | ~300 LAT |
| 8-core CPU (Good) | ~500 H/s | ~50 LAT | ~1,500 LAT |
| 16-core CPU (Great) | ~1000 H/s | ~100 LAT | ~3,000 LAT |
| 32-core Server (Excellent) | ~2000 H/s | ~200 LAT | ~6,000 LAT |

*Earnings depend on network difficulty and your CPU performance*

---

## 📖 Mining Commands

### **Start Mining**
```bash
# Auto-detect and use optimal threads
lattice-miner

# Or specify thread count (recommended: 75% of your CPU cores)
lattice-miner --threads 4
```

### **Check Your Balance**
```bash
lattice-cli wallet balance
```

### **Check Mining Status**
```bash
lattice-cli mining status
```

### **View Your Wallet Address**
```bash
lattice-cli wallet address
```

### **Stop Mining**
Just press `Ctrl+C` in the mining window.

---

## ⚙️ System Requirements

### **Minimum** (Can mine, but slower)
- **CPU:** 2 cores
- **RAM:** 4 GB
- **Disk:** 20 GB free space
- **Internet:** Any stable connection

### **Recommended** (Good mining performance)
- **CPU:** 4+ cores
- **RAM:** 8 GB
- **Disk:** 50 GB free space (SSD preferred)
- **Internet:** Broadband connection

### **Optimal** (Best mining performance)
- **CPU:** 8+ cores (more is better!)
- **RAM:** 16 GB
- **Disk:** 100 GB SSD
- **Internet:** Stable broadband
- **Cooling:** Good CPU cooling (mining generates heat)

---

## 💡 Mining Tips & Optimization

### **Maximize Your Earnings**
✅ **Use 75% of CPU cores** - Leave some for your system  
✅ **Keep your node synced** - Mining on unsynced node won't work  
✅ **Stable internet** - Lost connection = lost mining time  
✅ **Good cooling** - Better cooling = better performance  
✅ **Run 24/7** - More uptime = more rewards  

### **Things to Avoid**
❌ Don't use 100% CPU - system needs breathing room  
❌ Don't mine on laptops 24/7 - overheating risk  
❌ Don't forget to backup your wallet  
❌ Don't close miner during payouts  

### **Monitor Temperature**
Keep CPU temperature below 80°C (176°F) for optimal performance and hardware longevity.

---

## 🆘 Troubleshooting

### **Mining not starting?**
```bash
# Check if node is synced
lattice-cli node status

# If not synced, wait for sync to complete
# Mining will start automatically when synced
```

### **No rewards appearing?**
- Wait at least 10-15 minutes (blocks take time to mine)
- Check your mining status: `lattice-cli mining status`
- Ensure your wallet is created: `lattice-cli wallet address`

### **Low hashrate?**
- Increase thread count: `lattice-miner --threads 8`
- Close other programs to free up CPU
- Check CPU temperature (thermal throttling reduces performance)

### **Connection issues?**
```bash
# Test network connectivity
lattice-cli node peers

# If no peers, check firewall settings
# Port 30333 needs to be accessible
```

---

## 📚 More Information

- **📖 Complete Mining Guide:** [MINING_GUIDE.md](MINING_GUIDE.md)
- **🚀 Advanced Features:** [ADVANCED_FEATURES.md](ADVANCED_FEATURES.md)
- **📊 Project Status:** [FINAL_REPORT.md](FINAL_REPORT.md)
- **🐳 Docker Deployment:** [DEPLOYMENT.md](DEPLOYMENT.md)

---

## 🔐 Security & Wallet

### **Backup Your Wallet**
Your wallet file is located at: `~/.lattice/wallet.json`

**⚠️ IMPORTANT:** Back up this file regularly! If you lose it, you lose your LAT tokens forever.

```bash
# Backup your wallet
cp ~/.lattice/wallet.json ~/wallet-backup-$(date +%Y%m%d).json
```

### **Quantum-Resistant Security**
Lattice uses **CRYSTALS-Dilithium3** (NIST standard) for signatures, making your coins safe from quantum computers. Your LAT tokens are protected against future quantum attacks!

---

## 💬 Community & Support

- **🌐 Website:** https://latticechain.io
- **💬 Discord:** https://discord.gg/lattice
- **🐦 Twitter:** https://twitter.com/latticechain
- **📖 Docs:** https://docs.latticechain.io
- **🐛 GitHub:** https://github.com/dill-lk/Lattice

---

## 🎁 Bonus Features

### **Mining Pool Support** (Coming Soon)
Join mining pools to get more consistent payouts, even with lower hashrate.

### **Mobile Wallet** (Roadmap)
Check your balance and send transactions from your phone.

### **Staking** (Future)
Earn passive income by staking your LAT tokens.

---

## 🏆 Join the Network

**Be part of the quantum-resistant revolution!** Every miner helps secure the network and make cryptocurrency safer for the future.

Start mining today and earn LAT tokens while protecting the future of finance! 🚀⛏️💎

---

## 📝 For Developers

If you're a developer looking to build on Lattice or contribute to the codebase:

- **Build from source:** `cargo build --release`
- **Run tests:** `cargo test`
- **Architecture docs:** See `docs/` folder
- **Contributing:** See [CONTRIBUTING.md](CONTRIBUTING.md)
- **API docs:** See [lattice-rpc/README.md](crates/lattice-rpc/README.md)

---

## 📄 License

MIT OR Apache-2.0 - Free and open source forever! ❤️

