# Kanari RPC Server — API Usage

This document explains how to call the Kanari JSON-RPC API exposed by the RPC server (`/` and `/rpc` endpoints).

**Endpoint**

- **URL**: `http://<host>:<port>/rpc` (also available at `/`)
- **Method**: `POST`
- **Content-Type**: `application/json`

**Request wrapper**
All requests use the JSON-RPC 2.0 wrapper:

```json
{
  "jsonrpc": "2.0",
  "method": "kanari_<methodName>",
  "params": { /* method-specific fields */ },
  "id": 1
}
```

**Top-level behavior**

- Successful replies return a `RpcResponse` with `result` populated.
- Errors populate the `error` field with `code`, `message`, and optional `data`.

**Module-related methods**

- `kanari_publishModule` — publish a new Move module

Request `params` (PublishModuleRequest):

```json
{
  "sender": "0x...",
  "module_bytes": [1,2,3],
  "module_name": "MyModule",
  "gas_limit": 1000000,
  "gas_price": 1,
  "sequence_number": 0,
  "signature": null,
  "execute_immediate": false
}
```

Response (success):

```json
{
  "jsonrpc": "2.0",
  "result": { "hash": "<tx-hash-hex>", "status": "pending", "action": "publish" },
  "id": 1
}
```

- `kanari_upgradeModule` — upgrade an existing module (same params as publish)

Request `params` (UpgradeModuleRequest) is identical in shape to `PublishModuleRequest`.

Response (success):

```json
{ "jsonrpc": "2.0", "result": { "hash": "<tx-hash>", "status": "pending", "action": "upgrade" }, "id": 1 }
```

- `kanari_getModule` — fetch module info and bytecode

Request `params`:

```json
{ "address": "0x...", "name": "MyModule" }
```

Response `result` (ModuleInfo):

```json
{
  "address": "0x...",
  "name": "MyModule",
  "bytecode_hash": "<blake3-hex>",
  "size": 1234,
  "dependencies": []
}
```

- `kanari_listModules` — list all modules available in runtime

Request `params`: can be an empty object `{}`.

Response `result`: an array of `ModuleInfo` objects.

- `kanari_verifyModule` — verify module bytecode locally

Request `params`:

```json
{ "module_bytes": [1,2,3] }
```

Response (example valid):

```json
{ "jsonrpc": "2.0", "result": { "valid": true, "address": "0x...", "name": "MyModule" }, "id": 1 }
```

Response (invalid):

```json
{ "jsonrpc": "2.0", "result": { "valid": false, "error": "<verifier error>" }, "id": 1 }
```

**Transaction methods**

- `kanari_submitTransaction` - submit a signed transfer or burn transaction

Request `params` (SignedTransactionData):

```json
{
  "sender": "Ed25519:<public-key-hex>",
  "recipient": "0x...",
  "amount": 1000,
  "gas_limit": 1000000,
  "gas_price": 1,
  "sequence_number": 0,
  "signature": [1,2,3],
  "execute_immediate": true
}
```

When `execute_immediate` is `true`, the RPC server attempts execution immediately and returns the resulting changeset. When omitted or `false`, the transaction is submitted as pending.

**Examples (curl)**

Publish (submit pending tx):

```bash
curl -X POST http://127.0.0.1:19001/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"kanari_publishModule","params":{"sender":"0x1","module_bytes":[1,2,3],"module_name":"M","gas_limit":1000000,"gas_price":1,"sequence_number":0},"id":1}'
```

Get module:

```bash
curl -X POST http://127.0.0.1:19001/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"kanari_getModule","params":{"address":"0x1","name":"M"},"id":2}'
```

Verify module:

```bash
curl -X POST http://127.0.0.1:19001/rpc \
  -H 'Content-Type: application/json' \
  -d '{"jsonrpc":"2.0","method":"kanari_verifyModule","params":{"module_bytes":[1,2,3]},"id":3}'
```

**Notes & tips**

- All numeric byte arrays use JSON arrays of integers for `Vec<u8>` fields (e.g. `module_bytes`).
- `execute_immediate` is optional and, if true, will attempt to run the transaction immediately and return the changeset.
- Errors use the `RpcError` structure with standard JSON-RPC error codes for parse/invalid/method-not-found and custom module/transaction error codes for runtime failures.

For more details on the request/response types, see the API definitions in the crate: [crates/kanari-rpc-api/src/lib.rs](crates/kanari-rpc-api/src/lib.rs).
