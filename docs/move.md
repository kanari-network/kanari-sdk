# Move CLI Manual

The Move CLI is a comprehensive command-line interface for developing, testing, and deploying Move smart contracts on the Kanari blockchain platform.

## Table of Contents

- [Installation](#installation)
- [Basic Usage](#basic-usage)
- [Commands Overview](#commands-overview)
- [Development Workflow](#development-workflow)
- [Building and Testing](#building-and-testing)
- [Documentation and Analysis](#documentation-and-analysis)

## Installation

The Move CLI is included with the Kanari SDK. Ensure you have the `kari` binary in your PATH.

```bash
# Verify installation
kari --version
```

## Basic Usage

```bash
kari move <command> [options]
```

To see all available commands:

```bash
kari move
```

## Commands Overview

| Command | Description |
|---------|-------------|
| `new` | Create a new Move package |
| `build` | Build the Move package |
| `test` | Run Move unit tests |
| `publish` | Publish Move module to blockchain |
| `call` | Call a function in a deployed Move module |
| `doc` | Generate documentation |
| `info` | Print address information |
| `coverage` | Inspect test coverage |
| `disassemble` | Disassemble Move bytecode |
| `errmap` | Generate error map |
| `migrate` | Migrate Move module |
| `sandbox` | Execute sandbox commands |

## Development Workflow

### 1. Create New Project

Create a new Move package:

```bash
kari move new my_token_project
```

This creates a directory structure:
```
my_token_project/
├── Move.toml
├── sources/
│   └── example.move
└── tests/
    └── example_tests.move
```

**Example Move.toml:**
```toml
[package]
name = "MyTokenProject"
version = "1.0.0"

[dependencies]
MoveStdlib = { git = "https://github.com/move-language/move.git", subdir = "language/move-stdlib", rev = "main" }

[addresses]
std = "0x1"
my_addr = "0x42"
```

### 2. Write Move Code

**Example token module (sources/token.move):**
```move
module my_addr::token {
    use std::signer;
    
    struct Token has key {
        value: u64,
    }
    
    public fun mint(account: &signer, value: u64) {
        move_to(account, Token { value });
    }
    
    public fun get_value(addr: address): u64 acquires Token {
        borrow_global<Token>(addr).value
    }
}
```

## Building and Testing

### Build Package

Compile your Move code:

```bash
cd my_token_project
kari move build
```

**Build output:**
```
BUILDING MyTokenProject
Including dependency MoveStdlib
Compiling 1 modules in MyTokenProject
```

### Run Tests

Execute unit tests:

```bash
kari move test
```

**Example test file (tests/token_tests.move):**
```move
#[test_only]
module my_addr::token_tests {
    use my_addr::token;
    use std::signer;
    
    #[test(account = @0x1)]
    public fun test_mint(account: signer) {
        token::mint(&account, 100);
        assert!(token::get_value(signer::address_of(&account)) == 100, 0);
    }
}
```

### Test with Coverage

Run tests with coverage analysis:

```bash
kari move test --coverage
kari move coverage
```


## Documentation and Analysis

### Generate Documentation

Create HTML documentation for your modules:

```bash
kari move doc
```

**Generated files:**
```
doc/
├── index.html
├── my_addr_token.html
└── assets/
```

### Print Address Information

Display address mapping:

```bash
kari move info
```

### Disassemble Bytecode

View compiled bytecode:

```bash
kari move disassemble --interactive
```

### Generate Error Map

Create error code mapping:

```bash
kari move errmap
```



---

**Version**: 1.0.0  
**Last Updated**: 2024  
**For Support**: Refer to the Kanari SDK documentation or Move language documentation
