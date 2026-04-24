# Kanari Move Examples & Tutorials

Welcome to the Kanari Move examples and tutorials! This collection provides hands-on, practical examples for building on the Kanari blockchain using the Move programming language.

## 📚 Tutorial Series

### Beginner Tutorials

1. **[Creating Tokens](creating-coins.md)** - Learn how to create your first fungible token
   - Token creation with metadata
   - Minting and burning
   - Transferring tokens
   - Common patterns (fixed supply, capped, governance)

2. **[NFT Tutorial](nft-tutorial.md)** - Build NFT collections from scratch
   - Creating NFT collections
   - Minting with supply control
   - NFT marketplaces
   - Metadata management

3. **[DeFi Staking](defi-staking-tutorial.md)** - Implement staking protocols
   - Staking pool architecture
   - Reward calculation
   - Time-based incentives
   - Security considerations

### Reference Documentation

1. **[Usage Examples](usage-examples.md)** - Comprehensive module reference
   - Transfer operations
   - Coin management
   - Object lifecycle
   - Data structures (Table, Bag, Dynamic Fields)
   - Cryptography (Ed25519, ECDSA, Hash functions)
   - Transaction context
   - Mathematical operations

## 🚀 Quick Start

### Prerequisites

- Install the [Kanari SDK](../../README.md)
- Basic understanding of Move syntax
- Familiarity with blockchain concepts

### Running Examples

```bash
# Navigate to example directory
cd example_move/james

# Build the package
kanari move build

# Run tests
kanari move test

# Publish to local network
kanari move publish
```

## 📖 Learning Path

### For Beginners

1. Start with [Creating Tokens](creating-coins.md) to understand basic token mechanics
2. Move to [NFT Tutorial](nft-tutorial.md) to learn about unique assets
3. Study [Usage Examples](usage-examples.md) for comprehensive API reference

### For DeFi Developers

1. Review [DeFi Staking](defi-staking-tutorial.md) for protocol design
2. Explore advanced patterns in [Usage Examples](usage-examples.md)
3. Check out the [DEX example](../../example_move/dex_v1/)

### For NFT Projects

1. Follow [NFT Tutorial](nft-tutorial.md) for collection basics
2. Study marketplace patterns
3. Implement custom metadata systems

## 🎯 Example Categories

### Token Operations

- ✅ Create custom tokens
- ✅ Mint and burn
- ✅ Split and merge
- ✅ Transfer safely
- ✅ Update metadata

### NFT Management

- ✅ Create collections
- ✅ Control supply
- ✅ Mint with caps
- ✅ Marketplace listings
- ✅ Metadata attachment

### DeFi Primitives

- ✅ Staking pools
- ✅ Reward distribution
- ✅ Lock periods
- ✅ Auto-compounding
- ✅ Multi-pool support

### Data Structures

- ✅ Tables (homogeneous)
- ✅ Bags (heterogeneous)
- ✅ Dynamic fields
- ✅ Object relationships

### Cryptography

- ✅ Signature verification
- ✅ Hash functions
- ✅ Key recovery
- ✅ Address derivation

## 💡 Best Practices

### Code Quality

- Write comprehensive tests
- Use meaningful error codes
- Document public functions
- Follow naming conventions

### Security

- Validate all inputs
- Use access controls
- Prevent reentrancy
- Handle edge cases
- Get professional audits

### Gas Optimization

- Batch operations
- Minimize storage writes
- Avoid unnecessary loops
- Use efficient data structures

## 🔧 Common Patterns

### Witness Pattern

```move
struct MY_TOKEN has drop {}

let (cap, meta) = coin::create_currency<MY_TOKEN>(
    MY_TOKEN {}, // Consume witness
    // ... parameters
);
```

### Capability Pattern

```move
struct AdminCap has key, store {
    id: UID,
    is_admin: bool,
}

public fun admin_only(cap: &AdminCap) {
    assert!(cap.is_admin, 0);
    // Admin operations
}
```

### Freeze Pattern

```move
// Make object immutable
transfer::public_freeze_object(metadata);
```

## 📝 Contributing

We welcome contributions! Please see our [Contributing Guide](../../CONTRIBUTING.md) for details.

### Adding New Examples

1. Create your example in `example_move/`
2. Add documentation in this folder
3. Include comprehensive tests
4. Update this README
5. Submit a pull request

## 🆘 Getting Help

- **Documentation**: [Kanari Book](../book.toml)
- **API Reference**: [Module Docs](../../crates/kanari-frameworks/packages/kanari-system/docs/)
- **Community**: Join our Discord
- **Issues**: Report bugs on GitHub

## 📄 License

All examples are licensed under the Apache 2.0 License - see the [LICENSE](../../LICENSE) file for details.

## 🔗 Additional Resources

- [Move Language Documentation](https://move-language.github.io/move/)
- [Kanari Whitepaper](../../documentation/whitepaper/)
- [System Architecture](../../DOCS/SYSTEM_ER.md)
- [Security Guidelines](../../SECURITY.md)

---

**Happy Building! 🎉**

Start with [Creating Tokens](creating-coins.md) and build your first dApp on Kanari!
