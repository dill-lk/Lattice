# Lattice VM Scope (Phase 4 Baseline)

This document states what the current WASM runtime does and does not guarantee.

## What Exists

The `lattice-vm` crate already contains:
- WASM runtime creation
- contract deployment flow
- contract call flow
- static call support
- host functions for storage, balances, logs, hashing, and signature verification
- gas metering

## What Is Honest To Claim Today

The project can honestly claim:
- WASM smart-contract infrastructure exists
- deployment/call/runtime primitives are implemented
- host-function plumbing and gas accounting exist

## What Is Not Yet Fully True End-to-End

The project should not overclaim that the entire node / RPC / chain path is fully production-hardened for contracts yet.

Areas still being hardened:
- canonical on-chain persistence of deployed contract metadata across the full node flow
- tighter integration between node transaction execution and VM deployment/call semantics
- richer contract receipt / explorer / debugging support
- formal contract integration test coverage across node + RPC + state persistence

## Target Phase 4 Goal

Phase 4 should move Lattice from:
- "VM crate exists"

toward:
- "smart-contract lifecycle is consistent across runtime, node, RPC, and storage"

## Practical Guidance

Until the full hardening pass is complete:
- keep the whitepaper and README honest
- treat VM support as a serious alpha capability
- test deploy/call flows explicitly before making broader claims
