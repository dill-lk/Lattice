# Lattice Whitepaper

## Abstract

Lattice is a post-quantum blockchain built in Rust for long-term cryptographic resilience and practical operator usability. It uses NIST-standardized lattice cryptography, memory-hard Proof of Work, a persistent RocksDB state layer, JSON-RPC tooling, and a modular node architecture. The project is designed to be usable before post-quantum migration pressure becomes urgent, while remaining honest about current implementation scope.

---

## 1. Problem Statement

Most major blockchains still rely on classical public-key cryptography such as ECDSA or Ed25519. Sufficiently capable quantum computers would threaten those systems through Shor-style attacks against exposed public keys.

Lattice takes the opposite path:
- build quantum resistance into the chain from the start
- keep the stack understandable and self-hostable
- preserve fair participation through CPU-friendly mining

Lattice is therefore positioned as future-safe blockchain infrastructure rather than a retrofit after panic begins.

---

## 2. Cryptographic Foundation

### 2.1 Signatures

Lattice uses **CRYSTALS-Dilithium3** for transaction and account signatures.

Properties:
- NIST post-quantum standard
- security level: NIST level 3
- public key size: ~1952 bytes
- signature size: ~3309 bytes

### 2.2 Key Encapsulation

Lattice includes **CRYSTALS-Kyber768** primitives for post-quantum key exchange use cases.

### 2.3 Hashing

Lattice uses **SHA3-256** for:
- transaction hashing
- block hashing
- address derivation
- state hashing support

### 2.4 Address Format

Addresses are derived from the first 20 bytes of:

```text
SHA3-256(public_key)
```

They are exposed to users in Base58Check form.

---

## 3. Consensus Model

### 3.1 Proof of Work

Lattice uses an **Argon2id-based memory-hard Proof of Work** design.

Goals:
1. reduce ASIC advantage
2. keep mining more accessible on commodity hardware
3. make launch distribution fairer than heavily specialized mining ecosystems

### 3.2 Network Profiles

Current operating profiles:

| Network | Target Style | Memory Cost |
|---|---:|---:|
| Devnet | very fast local iteration | 512 KiB |
| Testnet | laptop-friendly validation | 4 MiB |
| Mainnet | full security profile | 64 MiB |

### 3.3 Difficulty Strategy

Lattice targets roughly **15-second blocks on mainnet** with a bootstrap period followed by dynamic adjustment.

---

## 4. Ledger and State Model

### 4.1 State

The chain tracks account state including:
- balance
- nonce
- code hash
- storage root

### 4.2 Transactions

Current transaction families:
- **Transfer**
- **Deploy**
- **Call**

### 4.3 Block Validation

Canonical block validation checks:
- structure limits
- parent linkage
- timestamp constraints
- transaction root correctness
- signature validity
- transaction semantics
- Proof of Work validity

### 4.4 State Root

Each block commits a `state_root` representing the post-execution world state after transactions and block rewards are applied.

---

## 5. Tokenomics

Lattice uses a fixed-supply model.

| Parameter | Value |
|---|---:|
| Symbol | LAT |
| Total Supply | 50,000,000 LAT |
| Decimals | 8 |
| Block Reward | 10 LAT |
| Genesis Allocation | 2,500,000 LAT |
| Minable Allocation | 47,500,000 LAT |

### 5.1 Genesis Allocation

The genesis allocation equals **5% of supply**:
- **500,000 LAT** immediately available
- **2,000,000 LAT** linearly vested over 24 months

### 5.2 Mining Allocation

The remaining **95%** is distributed through mining rewards.

### 5.3 Fee Model

Fees are denominated in the native smallest unit:

```text
1 LAT = 100,000,000 Latt
```

---

## 6. Node Architecture

The implementation is modular:

- `lattice-core` — types, validation, state, tokenomics
- `lattice-crypto` — Dilithium, Kyber, SHA3
- `lattice-consensus` — Argon2-based PoW
- `lattice-storage` — RocksDB-backed persistence
- `lattice-network` — libp2p protocol and sync primitives
- `lattice-vm` — WASM runtime
- `lattice-rpc` — JSON-RPC server
- `lattice-wallet` — keystore and transaction builder
- `lattice` — official all-in-one executable

---

## 7. Networking

Lattice is built around **libp2p** components for:
- gossip propagation
- request-response synchronization
- local discovery via mDNS

The repository also contains sharding and sync primitives. However, operators should treat sharded-network language as an architectural direction rather than a claim that full production shard routing is already complete in the running node path.

That distinction matters.

---

## 8. Smart Contracts

Lattice contains a WASM runtime with gas metering and host integration primitives.

Current project status should be described honestly as:
- **smart-contract infrastructure exists**
- **end-to-end production contract flow is still being hardened**

---

## 9. Design Philosophy

Lattice is built around five principles:

1. **future-safe cryptography**
2. **fairer participation**
3. **operator-first tooling**
4. **modular Rust implementation**
5. **honest implementation claims**

---

## 10. Current Status Statement

Lattice should currently be understood as:
- a real blockchain implementation
- a serious prototype / alpha system
- not yet a fully hardened mainnet chain

That is not a weakness in itself. It is the accurate status of an engineering system still being hardened.

---

## 11. Conclusion

Lattice exists to prove that a post-quantum blockchain can be built as practical infrastructure rather than speculative fearware. Its long-term thesis is simple:

> the world will eventually care about post-quantum migration,
> and chains built early, honestly, and credibly will have an advantage.

Lattice aims to be one of those chains.

---

## References

1. NIST Post-Quantum Cryptography Standardization Project
2. CRYSTALS-Dilithium specification
3. CRYSTALS-Kyber specification
4. RFC 9106 — Argon2 Memory-Hard Function
5. libp2p documentation
