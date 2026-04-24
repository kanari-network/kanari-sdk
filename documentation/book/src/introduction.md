# Introduction

Welcome to the Move programming language documentation for the Kanari blockchain! This guide will help you learn how to build secure, scalable decentralized applications using Move.

## What is Move?

Move is a programming language designed for digital assets and smart contracts. Originally developed by Meta for the Diem project, Move emphasizes safety and security through its unique type system and resource-oriented programming model.

### Key Features

- **Resource-Oriented Programming**: Digital assets behave like real-world resources that can't be copied or accidentally lost
- **Formal Verification**: Built-in tools for mathematically proving code correctness
- **Flexibility**: Supports both simple and complex decentralized applications
- **Security**: Type system prevents common vulnerabilities like double-spending

### Why Move for Blockchain?

Move was designed specifically for blockchains where security and asset safety are paramount:

```move
// In Move, this is impossible:
// let coin1 = my_coin;  // coin is moved
// let coin2 = my_coin;  // compile error! coin already moved

// This ensures no accidental duplication of assets
```

## Kanari Blockchain

Kanari is a high-performance blockchain built for decentralized finance (DeFi) applications. It uses Move as its smart contract language and provides additional features for building scalable applications.

### Kanari Advantages

- **High Performance**: Optimized for high transaction throughput
- **Low Fees**: Designed with zero user fees model
- **Developer Friendly**: Comprehensive tooling and documentation
- **Secure**: Built-in security features and best practices

## Getting Started

### Your First Move Module

```move
module my_project::hello {
    use kanari_system::tx_context::TxContext;
    use kanari_system::event;

    struct GreetingEvent has copy, drop {
        message: vector<u8>,
        sender: address,
    }

    public fun hello_world(ctx: &mut TxContext) {
        let sender = tx_context::sender(ctx);
        event::emit(GreetingEvent {
            message: b"Hello, Kanari!",
            sender,
        });
    }
}
```

This simple module demonstrates:

- Module declaration syntax
- Importing standard modules
- Defining custom event types
- Using transaction context
- Emitting events

### Key Concepts

1. **Modules**: Contain related functions and types
2. **Resources**: Special types that represent digital assets
3. **Transactions**: Calls to public functions
4. **Events**: Logs of important actions

## Move Language Structure

### Modules and Scripts

Move programs consist of two types:

- **Modules**: Libraries of functions and types, deployed once and used many times
- **Scripts**: One-time executable code (in older Move versions) or entry functions (in newer versions)

### Basic Syntax

Move syntax is similar to Rust:

```move
// Functions
public fun add(x: u64, y: u64): u64 {
    x + y
}

// Structs
struct Point has copy, drop {
    x: u64,
    y: u64,
}

// Conditionals
if (x > y) {
    x
} else {
    y
}

// Loops
let mut i = 0;
while (i < 10) {
    i = i + 1;
};
```

## Kanari Extensions

Kanari provides additional modules beyond standard Move:

- **coin**: For creating and managing tokens
- **transfer**: For moving resources between addresses
- **object**: For managing unique objects
- **tx_context**: For accessing transaction information
- **event**: For emitting logs
- **clock**: For time-based operations

## Safety Features

### Type Safety

Move's type system prevents many common bugs:

```move
let x: u64 = 10;
// let y: bool = x;  // Compile error!
let y: u64 = x;  // OK
```

### Resource Safety

The ownership system prevents double-spending:

```move
// This is impossible in Move:
// let coin1 = my_coin;
// let coin2 = my_coin;  // Error: my_coin already moved
```

### Access Control

Modules control access to their functions:

```move
public fun can_be_called_by_anyone() { }

fun can_only_be_called_internally() { }

public entry fun can_be_called_from_transactions() { }
```

## Learning Path

This documentation is structured to guide you from beginner to advanced Move development:

1. **Getting Started**: Basic concepts and simple examples
2. **Core Concepts**: Types, functions, and control flow
3. **Advanced Topics**: Resources, storage, and verification
4. **Practical Applications**: Building real applications

## Next Steps

Start with the [Modules and Scripts](modules-and-scripts.md) section to learn about organizing your Move code, then explore the specific topics that interest you most. Each section builds upon the previous ones, so consider following the documentation in order if you're new to Move.

For hands-on experience, try following the [Usage Examples](usage-examples.md) and tutorials like [Creating Coins](creating-coins.md) and [NFT Tutorial](nft-tutorial.md).

Happy coding on Kanari!
