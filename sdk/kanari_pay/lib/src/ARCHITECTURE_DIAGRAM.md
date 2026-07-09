# Architecture Diagram

## 📊 Module Dependency Flow

```mermaid
graph TB
    subgraph "Application Layer"
        UI[Flutter UI / App]
        CLI[CLI Tools]
    end

    subgraph "Facade Layer"
        Client[KanariClient<br/>197 lines]
    end

    subgraph "Module Layer"
        Transactions[TransactionOperations<br/>325 lines]
        Queries[QueriesModule<br/>184 lines]
        
        subgraph "Template Modules"
            Tokens[Token Module<br/>~150 lines]
            NFT[NFT Module<br/>~150 lines]
            DeFi[DeFi Module<br/>~150 lines]
        end
    end

    subgraph "Core Utilities"
        BCS[BcsSerializers<br/>68 lines]
        RPC[RpcUtils<br/>83 lines]
    end

    subgraph "External"
        HTTP[HTTP Client]
        Crypto[Kanari Crypto]
        BCS_Lib[BCS Library]
    end

    UI --> Client
    CLI --> Client
    
    Client --> Transactions
    Client --> Queries
    Client -.->|Future| Tokens
    Client -.->|Future| NFT
    Client -.->|Future| DeFi
    
    Transactions --> BCS
    Transactions --> RPC
    Transactions --> HTTP
    Transactions --> Crypto
    Transactions --> BCS_Lib
    
    Queries --> RPC
    Queries --> HTTP
    
    Tokens --> BCS
    Tokens --> RPC
    Tokens --> Queries
    
    NFT --> BCS
    NFT --> RPC
    NFT --> Queries
    
    DeFi --> BCS
    DeFi --> RPC
    DeFi --> Queries
```

## 🔄 Request Flow

### Query Operation (Read)

```mermaid
sequenceDiagram
    participant App as Application
    participant Client as KanariClient
    participant Queries as QueriesModule
    participant RPC as RpcUtils
    participant Node as Kanari Node

    App->>Client: getOwner(address)
    Client->>Queries: getOwner(address)
    Queries->>RPC: request('kanari_getOwner')
    RPC->>Node: HTTP POST
    Node-->>RPC: JSON Response
    RPC-->>Queries: Parsed Result
    Queries-->>Client: OwnerInfo
    Client-->>App: OwnerInfo
```

### Transaction Operation (Write)

```mermaid
sequenceDiagram
    participant App as Application
    participant Client as KanariClient
    participant TxOps as TransactionOperations
    participant Queries as QueriesModule
    participant Crypto as Kanari Crypto
    participant RPC as RpcUtils
    participant Node as Kanari Node

    App->>Client: transfer(wallet, recipient, amount)
    Client->>TxOps: transfer(...)
    TxOps->>Queries: getOwner(wallet.address)
    Queries-->>TxOps: OwnerInfo (sequence number + owned objects)
    
    TxOps->>TxOps: Select / consolidate coin objects, then prepare tx
    TxOps->>Crypto: blake3Hash(serialized tx)
    Crypto-->>TxOps: Hash
    TxOps->>Wallet: sign(hash)
    Wallet-->>TxOps: Signature
    
    TxOps->>RPC: request with signature
    RPC->>Node: HTTP POST
    Node-->>RPC: Transaction Result
    RPC-->>TxOps: TransactionResult
    TxOps-->>Client: TransactionResult
    Client-->>App: TransactionResult
```

## 📁 File Organization

```
lib/src/
│
├── 🎭 Facade Layer
│   └── kanari_client.dart (197 lines)
│       ├── Delegates to modules
│       ├── Maintains backward compatibility
│       └── Provides unified API
│
├── 🔧 Core Utilities (Shared by all modules)
│   ├── bcs_serializers.dart (68 lines)
│   │   ├── hexToBytes()
│   │   ├── encodeU64()
│   │   ├── normalizeAddress()
│   │   └── extractCoinTypeFromObjectType()
│   │
│   └── rpc_utils.dart (83 lines)
│       ├── request() - Generic RPC call
│       └── executeViewFunction() - View function helper
│
└── 📦 Feature Modules
    │
    ├── ✅ transactions/ (Implemented)
    │   ├── constants.dart - RPC methods, gas settings
    │   ├── operations.dart (305 lines)
    │   │   ├── publishModule()
    │   │   ├── transfer()
    │   │   ├── executeFunction()
    │   │   ├── burn()
    │   │   └── transferToken()
    │   └── transactions.dart - Barrel file
    │
    ├── ✅ queries.dart (184 lines) - Implemented
    │   ├── getAccount()
    │   ├── getBalance()
    │   ├── getTokenBalance()
    │   ├── getAllBalances()
    │   ├── getCheckpoint()
    │   ├── getCheckpointHeight()
    │   ├── getTransaction()
    │   ├── getStats()
    │   ├── getHealth()
    │   ├── getModule()
    │   ├── listModules()
    │   └── verifyModule()
    │
    ├── 📝 tokens/ (Template Ready)
    │   ├── constants.dart - Token package addresses, functions
    │   ├── operations.dart - createCurrency, mint, burn, split, merge
    │   ├── queries.dart - getSupply, getMetadata, getUserCoins
    │   └── tokens.dart - Barrel file
    │
    ├── 📝 nft/ (Template Ready)
    │   ├── constants.dart - Collection/NFT addresses, functions
    │   ├── operations.dart - createCollection, mintNft, transferNft
    │   ├── queries.dart - getCollectionDetails, getNftMetadata
    │   └── nft.dart - Barrel file
    │
    ├── 📝 defi/ (Template Ready)
    │   ├── constants.dart - DEX/Lending addresses, functions
    │   ├── operations.dart - swap, addLiquidity, removeLiquidity
    │   ├── queries.dart - getQuote, getReserves, getLpBalance
    │   └── defi.dart - Barrel file
    │
    └── modules.dart - Central barrel file
        └── Exports all modules
```

## 🎯 Module Communication Pattern

```mermaid
graph LR
    subgraph "Module Structure Pattern"
        Constants[constants.dart<br/>Static values]
        Operations[operations.dart<br/>Write operations]
        Queries[queries.dart<br/>Read operations]
        Barrel[module.dart<br/>Exports]
    end
    
    Constants --> Operations
    Constants --> Queries
    Operations --> Barrel
    Queries --> Barrel
    
    subgraph "Dependencies"
        Core[Core Utilities]
        Models[Data Models]
        Wallet[KanariWallet]
    end
    
    Operations --> Core
    Operations --> Models
    Operations --> Wallet
    Queries --> Core
    Queries --> Models
```

## 📈 Growth Strategy

```mermaid
graph TD
    Current[Current State<br/>✅ Core + Transactions + Queries]
    
    Phase1[Phase 1: Implement Templates]
    Phase2[Phase 2: Add New Features]
    Phase3[Phase 3: Advanced Modules]
    
    Current --> Phase1
    Phase1 --> Phase2
    Phase2 --> Phase3
    
    subgraph "Phase 1"
        T1[Implement Token Module]
        T2[Implement NFT Module]
        T3[Implement DeFi Module]
    end
    
    subgraph "Phase 2"
        F1[Governance Module]
        F2[Staking Module]
        F3[Identity Module]
    end
    
    subgraph "Phase 3"
        A1[Oracle Integration]
        A2[Cross-chain Bridge]
        A3[Advanced DeFi]
    end
    
    Phase1 --- T1 & T2 & T3
    Phase2 --- F1 & F2 & F3
    Phase3 --- A1 & A2 & A3
```

## 🔑 Key Principles

1. **Single Responsibility**: Each module has one clear purpose
2. **Separation of Concerns**: Read vs Write operations separated
3. **Dependency Injection**: Modules receive dependencies (url, client, queries)
4. **Barrel Files**: Clean exports for easy importing
5. **Backward Compatibility**: Existing API maintained through facade
6. **Template Pattern**: Consistent structure across all modules
7. **Core Reusability**: Shared utilities prevent code duplication

---

**Architecture Version**: 2.0.0  
**Last Updated**: 2026-05-12  
**Status**: Production Ready ✅
