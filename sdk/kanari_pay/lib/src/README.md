# 📚 Kanari SDK Documentation Index

Welcome to the Kanari SDK Modular Architecture documentation.

## 🎯 Getting Started

### For Beginners

1. **[Quick Reference Guide](./QUICK_REFERENCE.md)** - Quick reference guide with everything you need
2. **[ARCHITECTURE.md](./ARCHITECTURE.md)** - Understand the system overview
3. **[REFACTORING_SUMMARY.md](./REFACTORING_SUMMARY.md)** - See what has changed

### For Developers

1. **[MODULE_DEVELOPMENT_GUIDE.md](./modules/MODULE_DEVELOPMENT_GUIDE.md)** - Step-by-step module development guide
2. **[ARCHITECTURE_DIAGRAM.md](./ARCHITECTURE_DIAGRAM.md)** - Diagrams and visual explanations

---

## 📖 All Documentation

### 🏗️ Architecture & Design

| Document | Description | For Who |
|----------|-------------|---------|
| [ARCHITECTURE.md](./ARCHITECTURE.md) | Architecture overview, module status, design principles | Everyone |
| [ARCHITECTURE_DIAGRAM.md](./ARCHITECTURE_DIAGRAM.md) | Visual diagrams, flow charts, dependency graphs | Architects, Senior Devs |
| [REFACTORING_SUMMARY.md](./REFACTORING_SUMMARY.md) | What changed, why, and benefits | Project Managers, Devs |

### 🛠️ Development Guides

| Document | Description | For Who |
|----------|-------------|---------|
| [MODULE_DEVELOPMENT_GUIDE.md](./modules/MODULE_DEVELOPMENT_GUIDE.md) | How to create new modules (detailed tutorial) | Developers |
| [QUICK_REFERENCE.md](./QUICK_REFERENCE.md) | Quick lookup for APIs, patterns, examples | All Developers |

### 📦 Module Documentation

| Module | Status | Documentation |
|--------|--------|---------------|
| **Core** | ✅ Complete | See [core/](./core/) directory |
| **Transactions** | ✅ Complete | [transactions/](./modules/transactions/) |
| **Queries** | ✅ Complete | [queries.dart](./modules/queries.dart) |
| **Tokens** | 📝 Template | [tokens/](./modules/tokens/) |
| **NFT** | 📝 Template | [nft/](./modules/nft/) |
| **DeFi** | 📝 Template | [defi/](./modules/defi/) |
| **Escrow** | ✅ Complete | [escrow/](../escrow/) (separate) |

---

## 🚀 Learning Path

### Level 1: Beginner (Basic Usage)

```
1. Read QUICK_REFERENCE.md → Learn imports and basic usage
2. Try using client.getOwner(), client.transfer(), or client.transferWithCoinObject()
3. Read ARCHITECTURE.md → Understand structure
```

### Level 2: Intermediate (Adding Features)

```
1. Read MODULE_DEVELOPMENT_GUIDE.md → Learn how to create modules
2. Choose a template module (tokens/nft/defi)
3. Implement following the guide
4. Write tests
```

### Level 3: Advanced (System Design)

```
1. Read ARCHITECTURE_DIAGRAM.md → Understand deep architecture
2. Study design patterns used
3. Design custom modules
4. Contribute back to SDK
```

---

## 📂 Folder Structure

```
lib/src/
│
├── 📚 Documentation (you are here)
│   ├── README.md ← You are reading this file
│   ├── QUICK_REFERENCE.md
│   ├── ARCHITECTURE.md
│   ├── ARCHITECTURE_DIAGRAM.md
│   └── REFACTORING_SUMMARY.md
│
├── 🎭 Facade
│   └── kanari_client.dart
│
├── 🔧 Core Utilities
│   ├── core.dart
│   ├── bcs_serializers.dart
│   └── rpc_utils.dart
│
└── 📦 Modules
    ├── modules.dart
    ├── transactions/
    ├── queries.dart
    ├── tokens/ (template)
    ├── nft/ (template)
    ├── defi/ (template)
    └── escrow/ (separate)
```

---

## 💡 Usage Tips

### Quick Information Lookup

- Need API usage? → **[QUICK_REFERENCE.md](./QUICK_REFERENCE.md)**
- Want to add new feature? → **[MODULE_DEVELOPMENT_GUIDE.md](./modules/MODULE_DEVELOPMENT_GUIDE.md)**
- Want to understand architecture? → **[ARCHITECTURE.md](./ARCHITECTURE.md)**

### Navigation Tips

- Use `Ctrl+F` (or `Cmd+F`) to search in documents
- Every document has Table of Contents at the top
- Cross-references between documents use relative links

---

## 🆘 Need Help?

### Troubleshooting Steps

1. **Check Quick Reference** - Answer might be there
2. **Search in documentation** - Use search function
3. **Look at code examples** - In existing modules (escrow, transactions)
4. **Check errors** - Run `dart analyze` to see issues

### Common Questions

**Q: Where should I start?**  
A: Start with [QUICK_REFERENCE.md](./QUICK_REFERENCE.md) and try basic APIs

**Q: How to add new features?**  
A: Read [MODULE_DEVELOPMENT_GUIDE.md](./modules/MODULE_DEVELOPMENT_GUIDE.md) for step-by-step instructions

**Q: How to use template modules?**  
A: Open files in `modules/tokens/`, `modules/nft/`, or `modules/defi/` and implement according to comments

**Q: Are there breaking changes?**  
A: No! All original APIs still work. See [REFACTORING_SUMMARY.md](./REFACTORING_SUMMARY.md)

---

## 📊 Documentation Statistics

| Metric | Value |
|--------|-------|
| Total Documents | 5 files |
| Total Lines | ~2,000+ lines |
| Code Examples | 50+ examples |
| Diagrams | 5+ Mermaid diagrams |
| Templates | 3 module templates |

---

## 🔄 Updates & Changelog

### Version 2.0.0 (2026-05-12)

- ✅ Refactored to modular architecture
- ✅ Created template modules (tokens, nft, defi)
- ✅ Comprehensive documentation
- ✅ Backward compatible

### Previous Versions

- Version 1.x: Single-file architecture (deprecated)

---

## 🤝 Contributing

Want to improve documentation?

1. Fork repository
2. Edit documentation
3. Submit pull request
4. Update this index if adding new docs

---

## 📞 Contact & Support

- **Issues**: Report bugs or request features
- **Discussions**: Ask questions and share ideas
- **Documentation**: Help improve docs

---

## 🎓 Additional Resources

### External Links

- [Move Language Guide](https://move-language.github.io/move/)
- [BCS Serialization](https://github.com/diem/bcs)
- [Flutter Documentation](https://docs.flutter.dev/)
- [Dart Language Tour](https://dart.dev/guides/language/language-tour)

### Internal Resources

- [Kanari System Package Docs](../../../crates/kanari-frameworks/packages/kanari-system/docs/)
- [Example Move Contracts](../../../example_move/)
- [SDK Tests](../test/)

---

## ✨ Summary

**Kanari SDK v2.0.0** comes with:

- ✅ Scalable modular architecture
- ✅ Ready-to-use template modules
- ✅ Complete documentation (5 files)
- ✅ 100% backward compatible
- ✅ Developer-friendly guides

**Ready to start developing!** 🚀

---

**Documentation Version**: 2.0.0  
**Last Updated**: 2026-05-12  
**Maintained by**: Kanari Team  
**Status**: Production Ready ✅
