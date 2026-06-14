# Lattice Architecture

This document describes the current high-level architecture of Lattice after the unified CLI transition.

## 1. Product Entry Point

The official user-facing executable is:

```text
lattice
```

It provides:
- top-level quick actions (`lattice`, `--node`, `--mine`, `--wallet-new`, `--balance`, `--send`)
- advanced subcommands (`node`, `miner`, `wallet`, `tx`, `query`, `contract`, `doctor`, `chain`, `mempool`)

Legacy binaries remain compatibility wrappers only.

## 2. Workspace Modules

### Core modules
- `lattice-core` — blocks, transactions, state, validation, tokenomics
- `lattice-crypto` — Dilithium, Kyber, SHA3
- `lattice-consensus` — Argon2-based memory-hard PoW
- `lattice-storage` — RocksDB persistence, state snapshots, contract code storage
- `lattice-vm` — WASM runtime and host functions
- `lattice-rpc` — JSON-RPC server and handlers
- `lattice-network` — libp2p behaviour, peer manager, sync manager
- `lattice-wallet` — keystore and transaction builder

### Unified application layer
- `bins/lattice` — official all-in-one CLI / node / miner entrypoint

## 3. State Flow

Canonical state handling in the unified node path is:
1. load state snapshot from storage
2. validate block with core validation
3. execute transactions through core execution path
4. apply contract lifecycle effects (where integrated)
5. compute and verify state root
6. persist updated state
7. create snapshot for rollback / reorg support

## 4. Networking Flow

The unified node path now includes:
- libp2p swarm setup
- gossip topic subscription
- request-response sync wiring
- peer-manager integration
- sync-manager integration
- peer snapshots surfaced through RPC/operator UX

This is real networking integration, but it is still considered alpha-hardening territory rather than final production maturity.

## 5. RPC Layer

The RPC layer exposes:
- chain height / block lookup
- transaction lookup / receipt lookup
- balance / nonce queries
- mempool stats
- node/network status payloads
- mining work submission
- read-only VM-backed contract call path

## 6. Smart Contract Scope

Current honest statement:
- WASM runtime infrastructure exists
- read-only VM call plumbing exists through RPC
- deeper end-to-end deploy/call persistence is still being hardened

See also:
- `docs/vm-scope.md`
- `docs/rollback-strategy.md`
- `docs/protocol-baseline.md`

## 7. Architectural Status

Lattice is currently best described as:
- modular
- serious
- technically differentiated
- still in alpha hardening before trustworthy public-scale operation
