# Lattice JSON-RPC API Reference

## Overview

Lattice exposes a JSON-RPC 2.0 API for interacting with the blockchain. All methods are prefixed with `lat_`.

## Connection

Default endpoint: `http://localhost:8545`

## Methods

### Chain Information

#### `lat_blockNumber`

Returns the current block height.

**Parameters**: None

**Returns**: `string` - Block height as hex

**Example**:
```json
// Request
{"jsonrpc":"2.0","method":"lat_blockNumber","params":[],"id":1}

// Response
{"jsonrpc":"2.0","id":1,"result":"0x5b8d80"}
```

#### `lat_chainId`

Returns the chain ID.

**Parameters**: None

**Returns**: `string` - Chain ID as hex

**Example**:
```json
// Request
{"jsonrpc":"2.0","method":"lat_chainId","params":[],"id":1}

// Response
{"jsonrpc":"2.0","id":1,"result":"0x1"}
```

---

### Block Methods

#### `lat_getBlockByNumber`

Returns block by height.

**Parameters**:
1. `blockNumber`: `string` - Block height as hex, or `"latest"`, `"earliest"`, `"pending"`
2. `fullTransactions`: `boolean` - If true, return full tx objects; if false, return tx hashes

**Returns**: `object` - Block object

**Example**:
```json
// Request
{"jsonrpc":"2.0","method":"lat_getBlockByNumber","params":["0x1b4", true],"id":1}

// Response
{
  "jsonrpc": "2.0",
  "id": 1,
  "result": {
    "number": "0x1b4",
    "hash": "0x...",
    "parentHash": "0x...",
    "timestamp": "0x...",
    "miner": "0x...",
    "difficulty": "0x...",
    "stateRoot": "0x...",
    "transactionsRoot": "0x...",
    "transactions": [...]
  }
}
```

#### `lat_getBlockByHash`

Returns block by hash.

**Parameters**:
1. `blockHash`: `string` - 32-byte block hash
2. `fullTransactions`: `boolean` - If true, return full tx objects

**Returns**: `object` - Block object or `null`

---

### Account Methods

#### `lat_getBalance`

Returns account balance.

**Parameters**:
1. `address`: `string` - Account address
2. `blockNumber`: `string` - Block number or `"latest"`

**Returns**: `string` - Balance in smallest units (hex)

**Example**:
```json
// Request
{"jsonrpc":"2.0","method":"lat_getBalance","params":["1A2b3C...","latest"],"id":1}

// Response
{"jsonrpc":"2.0","id":1,"result":"0xde0b6b3a7640000"}
```

#### `lat_getTransactionCount`

Returns account nonce.

**Parameters**:
1. `address`: `string` - Account address
2. `blockNumber`: `string` - Block number or `"latest"`

**Returns**: `string` - Nonce as hex

---

### Transaction Methods

#### `lat_sendRawTransaction`

Submits a signed transaction.

**Parameters**:
1. `data`: `string` - Signed transaction data (hex)

**Returns**: `string` - Transaction hash

**Example**:
```json
// Request
{"jsonrpc":"2.0","method":"lat_sendRawTransaction","params":["0xf86c..."],"id":1}

// Response
{"jsonrpc":"2.0","id":1,"result":"0x...txhash..."}
```

#### `lat_getTransactionByHash`

Returns transaction by hash.

**Parameters**:
1. `txHash`: `string` - Transaction hash

**Returns**: `object` - Transaction object or `null`

#### `lat_getTransactionReceipt`

Returns transaction receipt.

**Parameters**:
1. `txHash`: `string` - Transaction hash

**Returns**: `object` - Receipt object with status, gas used, logs

---

### Contract Methods

#### `lat_call`

Executes a contract call (read-only, no state changes).

**Parameters**:
1. `callObject`: `object`
   - `to`: `string` - Contract address
   - `data`: `string` - Encoded function call
   - `from`: `string` (optional) - Caller address
2. `blockNumber`: `string` - Block number or `"latest"`

**Returns**: `string` - Encoded return value

#### `lat_estimateGas`

Estimates gas for a transaction.

**Parameters**:
1. `callObject`: `object` - Same as `lat_call`

**Returns**: `string` - Estimated gas as hex

---

### Mining Methods

#### `lat_mining`

Returns mining status.

**Parameters**: None

**Returns**: `boolean` - `true` if mining

#### `lat_hashrate`

Returns current hashrate.

**Parameters**: None

**Returns**: `string` - Hashes per second as hex

#### `lat_getWork`

Returns mining work template.

**Parameters**: None

**Returns**: `array`
1. `headerHash`: Current block header hash
2. `seedHash`: Seed for PoW
3. `target`: Difficulty target

#### `lat_submitWork`

Submits a mining solution.

**Parameters**:
1. `nonce`: `string` - Found nonce
2. `headerHash`: `string` - Header hash
3. `mixDigest`: `string` - Mix digest (for verification)

**Returns**: `boolean` - `true` if valid

---

## Error Codes

| Code | Message | Description |
|------|---------|-------------|
| -32700 | Parse error | Invalid JSON |
| -32600 | Invalid Request | Invalid request object |
| -32601 | Method not found | Unknown method |
| -32602 | Invalid params | Invalid parameters |
| -32603 | Internal error | Server error |
| 1 | Insufficient funds | Not enough balance |
| 2 | Nonce too low | Transaction nonce already used |
| 3 | Gas too low | Gas limit insufficient |
| 4 | Invalid signature | Signature verification failed |

---

## WebSocket Subscriptions

Connect to `ws://localhost:8546` for real-time updates.

#### `lat_subscribe`

Subscribe to events.

**Parameters**:
1. `subscriptionType`: `string`
   - `"newHeads"` - New block headers
   - `"newPendingTransactions"` - New pending txs
   - `"logs"` - Contract event logs

**Returns**: `string` - Subscription ID

#### `lat_unsubscribe`

Unsubscribe from events.

**Parameters**:
1. `subscriptionId`: `string`

**Returns**: `boolean` - Success
