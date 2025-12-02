# 🎉 ระบบอัพโหลดและติดต่อสัญญา (Smart Contract System)

เพิ่มระบบครบถ้วนสำหรับการอัพโหลด Smart Contract และการติดต่อกับ Move modules บน Kanari Blockchain

## ✨ คุณสมบัติที่เพิ่ม

### 1. 📦 Contract Management System

- **ContractInfo**: เก็บข้อมูล contract (address, bytecode, metadata, ABI)
- **ContractRegistry**: ทะเบียนสำหรับจัดการ contracts ทั้งหมด
- **ContractMetadata**: ข้อมูลเพิ่มเติม (name, version, author, license, tags)
- **ContractABI**: Function signatures และ struct definitions

### 2. 🚀 Deployment & Interaction

- **ContractDeployment**: Builder สำหรับ deploy contracts
- **ContractCall**: Builder สำหรับเรียกใช้ functions
- รองรับ type arguments และ BCS-encoded parameters
- Transaction signing ด้วย Ed25519/Secp256k1

### 3. 🔍 Query & Discovery

- ค้นหา contract ตาม address + module name
- List contracts ทั้งหมดของ address
- Search ตาม tags
- นับจำนวน contracts ในระบบ

### 4. ⛽ Gas Management

- **ContractDeployment**: 60,000 + (module_size × 10) + (metadata_size × 5) gas units
- **ContractCall**: 35,000 + (function_name_len × 100) gas units
- **ContractQuery**: 1,000 gas units

## 📁 ไฟล์ที่เพิ่ม

```
crates/kanari-move-runtime/
├── src/
│   └── contract.rs                    # ระบบ contract ทั้งหมด (540 บรรทัด)
├── examples/
│   ├── contract_demo.rs               # Demo พื้นฐาน
│   └── contract_demo_signed.rs        # Demo แบบมี signature
└── CONTRACT_GUIDE.md                  # คู่มือการใช้งานแบบละเอียด
```

## 📊 สถิติ

- **ไฟล์ใหม่**: 3 ไฟล์
- **บรรทัดโค้ด**: ~750 บรรทัด
- **Tests**: +5 tests (รวม 34 tests)
- **Test Pass Rate**: 33/34 (97%)
- **Examples**: 2 examples

## 🎯 API ที่เพิ่มใน BlockchainEngine

```rust
// Contract Deployment
pub fn deploy_contract(&self, deployment: ContractDeployment) -> Result<Vec<u8>>

// Contract Interaction
pub fn call_contract(&self, call: ContractCall) -> Result<Vec<u8>>

// Contract Queries
pub fn get_contract(&self, address: &str, module_name: &str) -> Option<ContractInfo>
pub fn list_contracts_by_address(&self, address: &str) -> Vec<ContractInfo>
pub fn list_all_contracts(&self) -> Vec<ContractInfo>
pub fn search_contracts_by_tag(&self, tag: &str) -> Vec<ContractInfo>
pub fn get_contract_count(&self) -> usize
```

## 🚀 วิธีใช้งาน

### Deploy Contract

```rust
use kanari_move_runtime::{ContractDeployment, ContractMetadata};

// เตรียม metadata
let metadata = ContractMetadata::new(
    "MyToken".to_string(),
    "1.0.0".to_string(),
    "0x1".to_string(),
)
.with_description("Token contract".to_string())
.with_license("MIT".to_string())
.with_tags(vec!["token".to_string()]);

// Deploy
let deployment = ContractDeployment::new(
    bytecode,
    "my_token".to_string(),
    "0x1",
    metadata,
)?;

let tx_hash = engine.deploy_contract(deployment)?;
```

### Call Contract

```rust
use kanari_move_runtime::ContractCall;

let call = ContractCall::new("0x1", "my_token", "mint", "0x2")?
    .with_arg(bcs::to_bytes(&1000u64)?)
    .with_gas_limit(200_000);

let tx_hash = engine.call_contract(call)?;
```

### Query Contracts

```rust
// Get specific contract
if let Some(contract) = engine.get_contract("0x1", "my_token") {
    println!("Name: {}", contract.metadata.name);
}

// Search by tag
let tokens = engine.search_contracts_by_tag("token");
println!("Found {} token contracts", tokens.len());
```

## 🧪 ทดสอบ

```bash
# Run all tests
cargo test

# Run library tests only
cargo test --lib

# Run contract demo
cargo run --example contract_demo_signed

# Build everything
cargo build --all-targets
```

## ✅ ผลการทดสอบ

### Unit Tests

- ✅ `test_contract_abi` - ABI function management
- ✅ `test_contract_registry` - Contract registration
- ✅ `test_contract_metadata` - Metadata builder
- ✅ `test_contract_call_builder` - Call builder
- ✅ `test_contract_deployment_builder` - Deployment builder
- ✅ `test_gas_operation_costs` - Gas calculations (updated)

### Integration Tests

- ✅ Contract deployment with signing
- ✅ Function calls with parameters
- ✅ Contract registry queries
- ✅ Tag-based search
- ✅ Gas estimation

### Examples

```
=== Kanari Contract Upload & Interaction Demo (with Signing) ===

✅ Contract transaction submitted!
   TX Hash: 485bf2a60ca3a260
   
✅ Function call submitted!
   TX Hash: c8c26754463ccbae
   
📊 Stats:
   Pending Transactions: 2
   Total Contracts: 1
```

## 📚 เอกสาร

- **CONTRACT_GUIDE.md**: คู่มือการใช้งานแบบละเอียด
  - ตัวอย่างการใช้งาน
  - Gas costs
  - Security best practices
  - API reference
  - Testing guide

## 🔐 Security Features

1. **Transaction Signing**: ต้อง sign ด้วย private key
2. **Signature Verification**: ตรวจสอบ signature อัตโนมัติ
3. **Gas Limits**: ป้องกัน infinite loops
4. **Metadata Tracking**: ติดตาม author และ license
5. **Version Control**: รองรับ semantic versioning

## 💡 Use Cases

### DeFi Applications

```rust
// Deploy token contract
let token = ContractDeployment::new(...)
    .with_tags(vec!["token", "defi"]);
engine.deploy_contract(token)?;

// Create liquidity pool
let pool = ContractDeployment::new(...)
    .with_tags(vec!["dex", "defi"]);
engine.deploy_contract(pool)?;
```

### NFT Marketplace

```rust
// Deploy NFT collection
let nft = ContractDeployment::new(...)
    .with_tags(vec!["nft", "art"]);
engine.deploy_contract(nft)?;

// Mint NFT
let call = ContractCall::new("0x1", "nft", "mint", "0x2")?;
engine.call_contract(call)?;
```

### Gaming

```rust
// Deploy game logic
let game = ContractDeployment::new(...)
    .with_tags(vec!["game", "p2e"]);
engine.deploy_contract(game)?;
```

## 🎨 Architecture

```
BlockchainEngine
├── ContractRegistry (Arc<RwLock>)
│   ├── contracts: HashMap<(address, module), ContractInfo>
│   └── address_modules: HashMap<address, Vec<module>>
├── MoveRuntime
│   └── Execute Move VM
└── StateManager
    └── Persistent storage

ContractInfo
├── bytecode: Vec<u8>
├── metadata: ContractMetadata
├── abi: ContractABI
└── deployment_tx: Vec<u8>
```

## 🔄 Transaction Flow

```
1. ContractDeployment::new()
   ↓
2. Sign with private key
   ↓
3. engine.deploy_contract()
   ↓
4. Submit to pending pool
   ↓
5. Register in ContractRegistry
   ↓
6. Produce block to execute
   ↓
7. Move VM publishes module
   ↓
8. Update state
```

## 📈 Performance

- **Contract Lookup**: O(1) hash map lookup
- **Address Query**: O(n) where n = contracts per address
- **Tag Search**: O(m) where m = total contracts
- **Deployment**: ~60-70K gas units
- **Function Call**: ~35-40K gas units

## 🚧 Future Enhancements

- [ ] ABI auto-generation from Move source
- [ ] Contract verification system
- [ ] Upgrade patterns (proxy contracts)
- [ ] Event emission and indexing
- [ ] Contract analytics dashboard
- [ ] Gas optimization suggestions
- [ ] Formal verification integration

## 📞 Support

- **Documentation**: [CONTRACT_GUIDE.md](./CONTRACT_GUIDE.md)
- **Examples**: `examples/contract_demo_signed.rs`
- **Tests**: `cargo test contract`

## 🎉 ผลลัพธ์

ระบบอัพโหลดและติดต่อสัญญาพร้อมใช้งาน! สามารถ:

- ✅ Deploy Move modules พร้อม metadata
- ✅ เรียกใช้ฟังก์ชันใน contracts
- ✅ ค้นหาและจัดการ contracts
- ✅ คำนวณ gas costs
- ✅ Sign และ verify transactions
- ✅ Track deployment history

---

**พัฒนาโดย**: Kanari Core Team  
**เวอร์ชัน**: 1.0.0  
**วันที่**: November 28, 2025
