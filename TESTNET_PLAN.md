# Lattice Testnet Plan

## Goal

Move from local validation and internal hardening into a small but honest public testnet.

## Preconditions

Before public testnet:
- compile and test suite stable locally and in CI
- multi-node local sync tested
- wallet / tx / mining loop validated
- rollback / snapshot behavior understood
- RPC status and diagnostics usable by external testers

## Stage 1 — Local Multi-Node Testnet

Run 3–5 nodes locally with:
- shared bootnode list
- one or more miners
- basic transaction traffic
- repeated restart / reconnect testing

Success criteria:
- peers discover / connect
- blocks propagate
- lagging node catches up
- miner and wallet flows still work during sync

## Stage 2 — Small Private Testnet

Invite a few trusted testers / friends to run nodes.

Success criteria:
- different machines can connect
- bootnode path works reliably
- tx propagation and block sync work across hosts
- diagnostics (`lattice doctor`, `lattice status`, `lattice peers`) are enough for support

## Stage 3 — Public Testnet

Publish:
- bootstrap nodes
- testnet reset policy
- known limitations
- faucet strategy
- getting-started guide

## Things To Measure

- node startup reliability
- peer counts and churn
- sync time from fresh start
- miner acceptance / rejection ratio
- mempool propagation behavior
- transaction confirmation time

## Exit Condition

Public testnet should run long enough to reveal:
- consensus bugs
- networking bugs
- state divergence bugs
- operator UX pain points
