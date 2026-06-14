# Lattice JSON-RPC API Reference

This document reflects the **current implemented RPC surface** used by the unified `lattice` stack.

Default endpoint:

```text
http://127.0.0.1:8545
```

All methods are prefixed with `lat_`.

---

## 1. Chain / Node Status

### `lat_blockNumber`
Returns the current best block height as hex.

**Params:** `[]`

**Example**
```json
{"jsonrpc":"2.0","method":"lat_blockNumber","params":[],"id":1}
```

---

### `lat_syncStatus`
Returns node-reported sync status.

**Params:** `[]`

**Returns**
- `syncing`: boolean
- `currentBlock`: hex string
- `highestBlock`: hex string

---

### `lat_peerInfo`
Returns peer snapshots currently visible to the node.

**Params:** `[]`

**Returns**
An array of objects with:
- `id`
- `address`
- `latency_ms`
- `score`

---

### `lat_nodeInfo`
Returns compact operator-oriented node information.

**Params:** `[]`

**Returns**
- `height`
- `syncing`
- `currentBlock`
- `highestBlock`
- `pendingTxs`
- `peerCount`

---

### `lat_networkInfo`
Returns network-facing operator information.

**Params:** `[]`

**Returns**
- `peerCount`
- `peers`
- `syncing`

---

## 2. Block Methods

### `lat_getBlockByNumber`
Returns a block by height.

**Params**
1. block number as hex, or tag: `latest`, `earliest`, `pending`
2. boolean: include full transaction objects

**Example**
```json
{"jsonrpc":"2.0","method":"lat_getBlockByNumber","params":["0x1", true],"id":1}
```

---

### `lat_getBlockByHash`
Returns a block by hash.

**Params**
1. block hash
2. boolean: include full transaction objects

---

## 3. Account Methods

### `lat_getBalance`
Returns account balance in base units (hex string).

**Params**
1. address

---

### `lat_getTransactionCount`
Returns account nonce / transaction count as hex.

**Params**
1. address
2. optional block tag (the CLI uses `latest`)

---

### `lat_mempoolStats`
Returns simple pending-transaction statistics.

**Params:** `[]`

**Returns**
- `pendingCount`
- `pendingHashes`

---

## 4. Transaction Methods

### `lat_sendRawTransaction`
Submits a signed transaction.

**Params**
1. hex-encoded signed transaction bytes

**Returns**
- transaction hash

---

### `lat_getTransactionByHash`
Returns a transaction object or `null`.

**Params**
1. transaction hash

---

### `lat_getTransactionReceipt`
Returns a transaction receipt object or `null`.

**Params**
1. transaction hash

---

## 5. Contract / Execution Methods

### `lat_call`
Executes a **read-only** VM-backed contract call.

**Params**
1. object with:
   - `to`: contract address
   - `data`: optional hex payload
   - `from`: optional caller address
   - `gas`: optional gas limit

**Returns**
- hex-encoded return bytes

**Notes**
- this currently uses the VM runtime in read-only mode
- the target contract must be known to the runtime mirror
- this is still alpha-hardening territory, not a final production contract RPC surface

---

### `lat_estimateGas`
Returns a simple gas estimate.

**Params**
1. call-like object

**Returns**
- gas estimate as hex string

---

## 6. Mining Methods

### `lat_getWork`
Returns a mining work template.

**Params**
1. coinbase address

**Returns**
An object containing:
- `workId`
- `txCount`
- `header`
  - `version`
  - `height`
  - `prevHash`
  - `txRoot`
  - `stateRoot`
  - `timestamp`
  - `difficulty`
  - `coinbase`

---

### `lat_submitWork`
Submits a mined nonce for a previously issued work template.

**Params**
1. object with:
   - `workId`
   - `nonce`
   - `powHash`

**Returns**
- boolean: accepted / rejected

---

## 7. Error Codes

| Code | Meaning |
|---|---|
| -32700 | parse error |
| -32600 | invalid request |
| -32601 | method not found |
| -32602 | invalid params |
| -32603 | internal error |
| -32001 | block not found |
| -32002 | transaction not found |
| -32003 | invalid transaction |
| -32004 | execution error |

RPC errors may also include structured `data` for specific failure reasons.

---

## 8. Important Scope Notes

The following are **not** currently documented here as stable features because they are not part of the current implemented JSON-RPC surface:
- websocket subscriptions
- `lat_chainId`
- `lat_mining`
- `lat_hashrate`

If they are added later, they should be documented here after implementation rather than before.
