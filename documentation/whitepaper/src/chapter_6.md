## 6. The Universal Payment Ecosystem

### 6.1 Developer Tools for All Industries

Kanari provides simple tools for building payment applications across all sectors:

**Tool Compatibility Matrix:**

| Platform | Language | Use Case |
|----------|----------|----------|
| Mobile Apps | Flutter/Dart | Digital wallets, payment apps |
| Web Applications | JavaScript/TypeScript | E-commerce, marketplaces |
| Backend Services | Rust | Payment processors, financial services |
| Enterprise Systems | REST API | ERP integration, banking systems |

### 6.2 Simple Integration Examples

**Mobile Wallet (Flutter/Dart):**

```dart
// Create payment wallet with Ed25519 curve
final wallet = await KanariWallet.generate(curve: KanariCurve.ed25519);

// Initialize client
final client = KanariClient('http://localhost:19001');

// Send instant payment
final result = await client.transfer(
  wallet: wallet,
  recipient: '0x9z8y7x6w5v4u3t2s1r0q9p8o7n6m5l4k3j2i1h0g9f8e7d6c5b4a3z2y1x0w9v8u',
  amount: 50000000, // 50 KANARI tokens
);

print('Instant payment sent! Transaction: ${result.hash}');
```

**Enterprise Integration (REST API):**

```bash
# Get account balance
curl -X POST http://localhost:19001 \
  -H "Content-Type: application/json" \
  -d '{
    "jsonrpc": "2.0",
    "method": "kanari_getBalance",
    "params": "0x1234567890123456789012345678901234567890123456789012345678901234",
    "id": 1
  }'

# Submit transaction (requires proper signature structure)
# Note: Direct REST submission requires BCS-serialized transaction with Blake3 hash signature
```

### 6.3 Payment Explorer

KanariScan provides real-time payment network visibility:

**Explorer Features:**

- Account balances and transaction history
- Cross-border payment tracking  
- Currency exchange rate monitoring
- Business analytics and reporting
- Smart contract verification

<!-- Accessible at: `soon` -->

### 6.4 Universal Development Workflow

**Simple 3-Step Process:**

1. **Build**: Create payment logic with Move smart contracts
2. **Test**: Run locally with testnet
3. **Deploy**: Launch to mainnet

**CLI Commands:**

```bash
# Create payment token
kanari move new usd_stablecoin

# Run build
kanari move build


# Test payment flows  
kanari move test 

# Deploy to production
kanari move publish 
```
