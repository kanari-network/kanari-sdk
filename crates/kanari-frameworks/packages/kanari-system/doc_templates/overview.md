# Kanari System Modules

This is the root document for the Kanari System module documentation. The Kanari System provides Move modules for blockchain operations including transfers, token management, and system utilities.

## Overview

The Kanari System is built on top of the Move programming language, providing a secure and efficient platform for decentralized applications. This documentation covers all modules available in the Kanari System.

## Features

- **Transfer Module**: Production-ready transfer functionality with full validation
- **Type Safety**: Strong type checking and validation
- **Security**: Built-in error handling and validation
- **Performance**: Optimized for efficient execution

## Index

> {{move-index}}

## Getting Started

To use Kanari System modules in your Move code, add the following to your `Move.toml`:

```toml
[dependencies]
KanariSystem = { local = "../kanari-system" }
```

Then import the modules you need:

```move
use kanari_system::transfer;
```

## Support

For issues and questions, please visit our GitHub repository.
