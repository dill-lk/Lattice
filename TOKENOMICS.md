# Lattice Tokenomics 🪙

## Overview

Lattice (LAT) is a quantum-resistant cryptocurrency with a carefully designed token economy that balances developer sustainability with community trust.

## Core Parameters

| Parameter | Value | Notes |
|-----------|-------|-------|
| **Symbol** | LAT | Lattice Token |
| **Total Supply** | 50,000,000 LAT | Fixed maximum supply |
| **Decimals** | 8 | 1 LAT = 100,000,000 Latt |
| **Smallest Unit** | Latt | Like Satoshi for Bitcoin |
| **Block Reward** | 10 LAT | Mining reward per block |
| **Target Block Time** | 15 seconds | ~5,760 blocks per day |

## Developer Genesis Fee (5% of Total Supply)

The Genesis Fee ensures sustainable development while maintaining community trust through transparent vesting.

### Allocation Breakdown

| Category | Amount | % of Supply | Availability |
|----------|--------|-------------|--------------|
| **Immediate** | 500,000 LAT | 1% | At genesis |
| **Vesting** | 2,000,000 LAT | 4% | 24-month linear vesting |
| **Total** | 2,500,000 LAT | 5% | - |

### Founder Wallet

```
Address: 13jXqXbCSghDF2KgyFQdtw8SvbJvpEyhft
Keystore ID: 7e73a394-12e4-4f41-816f-558c214d8a78
```

### Vesting Schedule

The 2,000,000 LAT vesting allocation is released linearly over 24 months:

```
Month 1:  ~83,333 LAT released (cumulative: 83,333)
Month 2:  ~83,333 LAT released (cumulative: 166,666)
Month 3:  ~83,333 LAT released (cumulative: 250,000)
...
Month 12: ~83,333 LAT released (cumulative: 1,000,000)
...
Month 24: ~83,333 LAT released (cumulative: 2,000,000)
```

**Key Benefits:**
- ✅ No cliff period - tokens start vesting immediately
- ✅ Linear release - predictable and fair
- ✅ Long duration - shows long-term commitment
- ✅ Transparent - all tracked on-chain

### Purpose of Immediate Allocation (500,000 LAT)

The immediately available funds are designated for:

1. **Infrastructure Costs** - Server hosting, cloud services
2. **Exchange Listings** - Listing fees for major exchanges
3. **Security Audits** - Professional code audits
4. **Legal & Compliance** - Legal structure setup
5. **Initial Marketing** - Launch campaigns

## Mining Economics

### Supply Distribution

```
Total Supply:          50,000,000 LAT (100%)
├── Genesis Allocation: 2,500,000 LAT (5%)
│   ├── Immediate:        500,000 LAT (1%)
│   └── Vesting:        2,000,000 LAT (4%)
└── Minable Supply:    47,500,000 LAT (95%)
```

### Mining Timeline

At 10 LAT per block:

| Milestone | Blocks | Time (approx) | Mined |
|-----------|--------|---------------|-------|
| First year | 2,102,400 | 1 year | 21,024,000 LAT |
| Second year | 4,204,800 | 2 years | 42,048,000 LAT |
| Max supply reached | 4,750,000 | ~2.26 years | 47,500,000 LAT |

**Note:** After approximately 2.26 years, all minable LAT will be distributed. After this point, miners earn only transaction fees.

## Supply Schedule

```
Year 0 (Genesis):
├── Circulating: 500,000 LAT (immediate allocation)
└── Locked: 2,000,000 LAT (vesting)

Year 1:
├── Mined: ~21,024,000 LAT
├── Vested: ~1,000,000 LAT (50% of vesting)
└── Total Circulating: ~22,524,000 LAT

Year 2:
├── Mined: ~42,048,000 LAT (capped at 47,500,000)
├── Vested: 2,000,000 LAT (100% complete)
└── Total Circulating: ~49,500,000 LAT

Year 2.26+:
└── Total Circulating: 50,000,000 LAT (max supply)
```

## Comparison with Other Projects

| Project | Premine/Genesis | Vesting | Notes |
|---------|----------------|---------|-------|
| **Bitcoin** | 0% | N/A | Pure mining |
| **Ethereum** | ~12% | None | ICO distribution |
| **Zcash** | 10% | 4 years | "Founders Reward" |
| **Lattice** | 5% | 2 years | Developer Genesis Fee |

Lattice takes a balanced approach with a modest allocation and transparent vesting.

## Code Implementation

The tokenomics are implemented in:

- **`crates/lattice-core/src/tokenomics.rs`** - Constants and utilities
- **`crates/lattice-core/src/genesis.rs`** - Genesis state creation
- **`crates/lattice-core/src/validation.rs`** - Block reward application

### Key Constants

```rust
// Token basics
pub const TOKEN_SYMBOL: &str = "LAT";
pub const DECIMALS: u8 = 8;
pub const LATT_PER_LAT: Amount = 100_000_000;

// Supply
pub const TOTAL_SUPPLY: Amount = 50_000_000 * LATT_PER_LAT;
pub const BLOCK_REWARD: Amount = 10 * LATT_PER_LAT;

// Genesis allocation
pub const TOTAL_GENESIS_ALLOCATION: Amount = 2_500_000 * LATT_PER_LAT;
pub const FOUNDER_IMMEDIATE_AMOUNT: Amount = 500_000 * LATT_PER_LAT;
pub const FOUNDER_VESTING_AMOUNT: Amount = 2_000_000 * LATT_PER_LAT;
pub const VESTING_DURATION_MONTHS: u64 = 24;
```

## Verifying On-Chain

You can verify the genesis allocation and vesting using:

```bash
# Check founder balance
lattice-cli wallet balance --address 13jXqXbCSghDF2KgyFQdtw8SvbJvpEyhft

# View genesis block
lattice-cli query block 0

# Check vesting status (future feature)
lattice-cli query vesting --address 13jXqXbCSghDF2KgyFQdtw8SvbJvpEyhft
```

## Security Considerations

1. **Vesting Contract** - Vesting tokens are not spendable until they vest
2. **Transparent Tracking** - All allocations visible on block explorer
3. **No Hidden Allocation** - Genesis state is deterministic and verifiable
4. **Quantum Resistant** - All keys use CRYSTALS-Dilithium signatures

## FAQ

**Q: Why 5% and not less?**
A: 5% provides adequate runway for development, exchange listings, marketing, and security audits while remaining well below industry averages (often 10-20%).

**Q: Can the founder dump all tokens at once?**
A: Only 500,000 LAT (1%) is immediately available. The remaining 2,000,000 LAT vests linearly over 24 months.

**Q: What happens to the vesting tokens if the project fails?**
A: Vesting tokens can only be claimed by the founder wallet. If the project is abandoned, unvested tokens effectively become inaccessible.

**Q: Is there a cliff period?**
A: No cliff. Tokens start vesting from block 0 and continue linearly for 24 months.

---

*Last Updated: March 2025*
*Version: 1.0*
