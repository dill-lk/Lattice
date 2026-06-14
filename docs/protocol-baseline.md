# Lattice Protocol Baseline

This document defines the current canonical behavior that code, docs, RPC templates, and operator tooling should follow.

## 1. Transaction Identity and Signing

- transaction signatures are verified with Dilithium public keys
- the sender address must match `Address::from_public_key(public_key)`
- signature verification must validate the canonical `signing_bytes()` payload

## 2. Transaction Root

All block templates and validators must compute:

```text
tx_root = Block::calculate_tx_root(transactions)
```

No alternative ad-hoc concatenation schemes are canonical.

## 3. State Root

A block's `state_root` must represent the world state after:
1. all block transactions are executed in order
2. fees are accumulated
3. the block reward plus fees are credited to coinbase

## 4. Block Acceptance

A block is only acceptable when:
- core block validation succeeds
- PoW verification succeeds
- the computed post-execution state root matches the header state root
- the resulting state is persisted successfully

## 5. Tokenomics Baseline

Current canonical economics:
- symbol: LAT
- decimals: 8
- total supply: 50,000,000 LAT
- block reward: 10 LAT
- genesis allocation: 2,500,000 LAT
- minable allocation: 47,500,000 LAT

## 6. Wallet Safety Baseline

For Dilithium accounts, public keys cannot be safely reconstructed from raw secret-key bytes alone.

Therefore:
- keystore import/export is the safe path
- combined key material can be reconstructed if both public and secret material are present
- raw secret-only reconstruction must fail loudly rather than silently generating a different wallet

## 7. Truth Policy

If code and docs disagree, the code path in the unified `lattice` stack is the source of truth until docs are updated.
