# Lattice Rollback / Reorg Strategy (Phase 4 Baseline)

This document defines the intended rollback and chain-reorganisation strategy for Lattice.

## Current Reality

The current unified node path validates and applies blocks through the core execution path and persists state snapshots.

What exists today:
- block import with post-execution state-root checking
- state snapshots keyed by block height
- block storage by hash and height

What is not fully implemented yet:
- full competing-fork selection logic
- automatic rollback / reapply of alternate branches
- finalized mainnet-grade reorg policy

## Strategy Direction

When full reorg support is implemented, the node should:

1. detect that an incoming branch competes with the current tip
2. find the common ancestor
3. rollback state to the ancestor snapshot
4. apply the stronger / winning branch block-by-block
5. restore mempool entries that were valid on the previous branch but not included on the new branch

## Winning Rule (Planned)

A branch should win based on the chain's canonical consensus rule, typically:
- greatest cumulative work

Simple height alone is not enough for production.

## Snapshot Use

State snapshots exist specifically to make rollback safe and deterministic.

Recommended reorg flow:
- snapshot at accepted heights
- keep a rolling retention window
- rollback using `StateStore::rollback_to_snapshot(height)`
- replay winning branch deterministically through core execution

## Safety Requirements

Before mainnet-grade reorg support is considered ready:
- rollback must preserve balance / nonce correctness
- mempool recovery must avoid duplicate or invalid pending transactions
- RPC views must refresh after rollback/reapply
- chain tip / sync status must update atomically with accepted branch state

## Phase 4 Meaning

For now, this document is the baseline plan that keeps rollback/reorg handling explicit instead of implicit.
