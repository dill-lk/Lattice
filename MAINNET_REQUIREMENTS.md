# Lattice Mainnet Requirements

Mainnet should not be treated as a branding milestone. It should be treated as a trust milestone.

## Required Before Mainnet

### 1. Technical Correctness
- canonical block execution stable
- contract lifecycle behavior clearly defined
- reorg / rollback policy implemented or explicitly constrained
- state snapshots verified under realistic use

### 2. Networking Confidence
- multi-node sync proven repeatedly
- bootnode flow stable
- peer reporting believable
- restart / reconnect scenarios tested

### 3. Operator Readiness
- `lattice` CLI stable
- installation / update path clear
- diagnostics sufficient for support
- config workflow understandable

### 4. Documentation Truth
- README, whitepaper, tokenomics, architecture docs aligned
- no claims ahead of implementation
- public limitations stated clearly

### 5. Testing & CI
- workspace build stable
- clippy stable
- integration tests reliable
- release packaging verified

### 6. Security Review
- wallet safety reviewed
- consensus and RPC reviewed
- VM/runtime review at least baseline-complete

## Strongly Recommended
- public testnet history
- explorer or at least robust chain inspection tooling
- backup / restore procedures documented
- operational runbooks for node maintainers

## Anti-Requirement

Do **not** launch mainnet just because:
- the codebase feels big
- the CLI looks polished
- the branding is cool
- the idea is strong

Mainnet needs operational confidence, not just vision.
