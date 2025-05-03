# Kanari Move API Guide

This guide explains how to use the Move Virtual Machine (VM) integration in the Kanari blockchain system.

## Table of Contents
1. [Introduction](#introduction)
2. [Deploying Move Modules](#deploying-move-modules)
3. [Executing Module Functions](#executing-module-functions)
4. [Module Information and Management](#module-information-and-management)
5. [Transaction Format](#transaction-format)
6. [Gas System](#gas-system)
7. [API Reference](#api-reference)
8. [Examples](#examples)

## Introduction

Kanari blockchain integrates the Move Virtual Machine to support smart contracts written in the Move programming language. Move is a safe and secure programming language designed for blockchain systems, initially developed by Facebook (now Meta) for the Libra/Diem blockchain.

### Key Features

- **Type Safety**: Move provides strong type safety and resource-aware programming.
- **Resource Ownership**: First-class resources that can only be moved, not copied.
- **Module System**: Structured code organization with separation of logic and data.
- **Formal Verification**: Designed to be amenable to formal verification.

## Deploying Move Modules

### Prerequisites

- Install Kanari CLI tools
- Create a Move package with `kari move new my_package`
- Set up module code in `sources` directory

### Deployment Process

To deploy a Move module to the Kanari blockchain:

```bash
kari move publish --address=0x123... [--gas-budget=3000000] [--skip-verify]
```

Parameters:
- `--address`: The blockchain address where the module will be deployed
- `--gas-budget`: Optional gas limit for the deployment transaction
- `--skip-verify`: Skip module verification (not recommended for production)

### Deployment Responses

Upon successful deployment, the system returns a JSON response containing:
- Transaction ID
- Gas used
- Module information including public functions
- Blockchain block height

## Executing Module Functions

Once deployed, you can execute functions in your modules using the RPC API:

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "vm_execute",
  "params": {
    "module_id": "0x123...::my_module",
    "function": "my_function",
    "args": [42, "hello"],
    "gas_budget": 10000
  },
  "id": 1
}' http://your.node.address:30030
```

Parameters:
- `module_id`: Full module identifier in format `0x{address}::{module_name}`
- `function`: Function name to execute
- `args`: Array of arguments to pass to the function
- `gas_budget`: Maximum gas to spend on execution
- `sender`: Optional sender address (defaults to system address)

## Module Information and Management

### Listing Deployed Modules

To view all deployed modules:

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "vm_list_modules",
  "params": [],
  "id": 1
}' http://your.node.address:30030
```

### Getting Module Details

To get information about a specific module:

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "vm_get_module",
  "params": {
    "module_id": "0x123...::my_module"
  },
  "id": 1
}' http://your.node.address:30030
```

## Transaction Format

When submitting a direct Move VM transaction through the blockchain:

```
data field format: "VM:{module_id}:{function}:{gas_budget}"
```

Example:
```
"VM:0x123...::counter:increment:1000000"
```

## Gas System

The Move VM uses a gas system to:
- Prevent infinite loops and other resource abuse
- Allocate costs to computation and storage operations
- Prioritize transactions in congested networks

Gas is calculated based on:
- Bytecode size and complexity
- Input data size
- Network congestion
- Storage operations

## API Reference

### vm_execute

Executes a function in a deployed module.

**Parameters:**
```json
{
  "module_id": "0x{address}::{module_name}",
  "function": "function_name",
  "args": [...],
  "gas_budget": 10000,
  "sender": "0x..." // optional
}
```

**Response:**
```json
{
  "status": "success",
  "tx_id": "vm_tx_...",
  "module": "0x123...::my_module",
  "function": "my_function",
  "gas_used": 420,
  "gas_display": "0.000000420",
  "execution_time_ms": 52,
  "block_height": 1234,
  "timestamp": 1698765432
}
```

### vm_get_module

Gets information about a deployed module.

**Parameters:**
```json
{
  "module_id": "0x{address}::{module_name}"
}
```

**Response:**
```json
{
  "module_id": "0x123...::my_module",
  "name": "my_module",
  "address": "0x123...",
  "public_functions": ["function1", "function2"],
  "size_bytes": 1024,
  "deploy_block_height": 1000,
  "current_block_height": 1234,
  "blocks_since_deploy": 234
}
```

### vm_list_modules

Lists all deployed modules.

**Parameters:**
```json
{}
```

**Response:**
```json
{
  "modules": [
    {
      "module_id": "0x123...::module1",
      "name": "module1",
      "address": "0x123...",
      "public_functions": ["function1", "function2"],
      "size_bytes": 1024,
      "deploy_block_height": 1000
    },
    // ...more modules
  ],
  "count": 5,
  "last_execution": 1698765432,
  "execution_count": 42
}
```

## Examples

### Counter Module

1. **Create a Counter module**

```move
module 0x1::counter {
    struct Counter has key {
        value: u64,
    }

    public fun init(account: &signer) {
        move_to(account, Counter { value: 0 });
    }

    public fun increment(account: &signer) acquires Counter {
        let counter = borrow_global_mut<Counter>(std::signer::address_of(account));
        counter.value = counter.value + 1;
    }

    public fun get_value(addr: address): u64 acquires Counter {
        borrow_global<Counter>(addr).value
    }
}
```

2. **Deploy the module**

```bash
kari move publish --address=0x123...
```

3. **Initialize counter**

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "vm_execute",
  "params": {
    "module_id": "0x123...::counter",
    "function": "init",
    "args": [],
    "sender": "0x123..."
  },
  "id": 1
}' http://your.node.address:30030
```

4. **Increment counter**

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "vm_execute",
  "params": {
    "module_id": "0x123...::counter",
    "function": "increment",
    "args": [],
    "sender": "0x123..."
  },
  "id": 1
}' http://your.node.address:30030
```

5. **Get counter value**

```bash
curl -X POST -H "Content-Type: application/json" -d '{
  "jsonrpc": "2.0",
  "method": "vm_execute",
  "params": {
    "module_id": "0x123...::counter",
    "function": "get_value",
    "args": ["0x123..."],
    "sender": "0x123..."
  },
  "id": 1
}' http://your.node.address:30030
```

## Best Practices

1. **Gas Management**
   - Always specify a reasonable gas budget for operations
   - Account for network congestion in high-traffic scenarios

2. **Module Design**
   - Keep modules focused and single-purpose
   - Use proper visibility modifiers (public/private)
   - Implement proper access controls for sensitive operations

3. **Error Handling**
   - Always check return values and handle errors gracefully
   - Use meaningful error codes in your modules

4. **Testing**
   - Test modules thoroughly with unit tests before deployment
   - Use `kari move test` to verify functionality

5. **Security**
   - Follow Move security best practices
   - Consider formal verification for critical contracts
   - Limit privileged operations to specific signers
