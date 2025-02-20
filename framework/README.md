# Kanari Framework

This directory contains the core Move framework packages for the Kanari blockchain platform.

## Package Structure

```text
framework/
├── packages/
│   ├── move-stdlib/       # Move standard library implementation
│   │   ├── sources/       # Standard library source files
│   │   └── tests/        # Standard library tests
│   └── kanari-framework/ # Kanari-specific framework
│       ├── sources/      # Framework source files
│       └── tests/        # Framework tests

```

## Move Standard Library

The `move-stdlib` package provides fundamental types and functions used across Move modules:
- Basic types (Vector, Option, String)
- Mathematical operations
- Hash functions
- Error handling

## Kanari Framework

The `kanari-framework` package implements Kanari-specific functionality:
- Account management
- Asset handling
- NFT standards
- Staking mechanisms
- Governance features

## Building the Framework

```bash
cd framework
kari move build
```

## Running Tests

```bash
cd framework
kari move test
```

## Documentation

Generate documentation for the framework:

```bash
cd framework
kari move doc
```

Documentation will be available in the `docs/` directory.

## Contributing

Please ensure all contributions:
1. Include appropriate tests
2. Follow Move coding standards
3. Include documentation updates
4. Pass all existing tests

## License

Copyright © 2024 Kanari Network
All rights reserved.