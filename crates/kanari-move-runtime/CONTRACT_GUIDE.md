# Kanari Contract System Guide

ระบบอัพโหลดและติดต่อสัญญา (Smart Contract) บน Kanari Blockchain

## 📦 ภาพรวม

Kanari รองรับการอัพโหลดและเรียกใช้ Smart Contract ที่เขียนด้วย Move language พร้อมระบบจัดการ Contract Registry และ ABI (Application Binary Interface)

## 🎯 คุณสมบัติหลัก

### 1. Contract Deployment (การอัพโหลดสัญญา)

- อัพโหลด Move modules ไปยัง blockchain
- บันทึก metadata (ชื่อ, version, author, license)
- ติดตาม deployment transaction hash
- คำนวณ gas สำหรับ deployment

### 2. Contract Interaction (การติดต่อสัญญา)

- เรียกใช้ฟังก์ชันใน contract
- ส่ง type arguments และ parameters
- รองรับการ sign transactions
- Gas metering แบบ real-time

### 3. Contract Registry (ทะเบียนสัญญา)

- เก็บข้อมูล contracts ทั้งหมด
- ค้นหาตาม address, module name, tag
- แสดง ABI และ function signatures
- Track deployment history

## 📝 โครงสร้างข้อมูล

### ContractInfo

```rust
pub struct ContractInfo {
    pub address: String,           // ที่อยู่ผู้เผยแพร่
    pub module_name: String,       // ชื่อ module
    pub bytecode: Vec<u8>,         // bytecode ของ module
    pub deployment_tx: Vec<u8>,    // transaction hash
    pub deployed_at: u64,          // block height
    pub abi: ContractABI,          // function signatures
    pub metadata: ContractMetadata, // ข้อมูลเพิ่มเติม
}
```

### ContractMetadata

```rust
pub struct ContractMetadata {
    pub name: String,              // ชื่อ contract
    pub version: String,           // เวอร์ชัน
    pub author: String,            // ผู้เขียน
    pub description: String,       // คำอธิบาย
    pub source_url: Option<String>, // URL source code
    pub license: Option<String>,   // ใบอนุญาต
    pub tags: Vec<String>,         // tags สำหรับค้นหา
}
```

### ContractABI

```rust
pub struct ContractABI {
    pub functions: Vec<FunctionSignature>, // ฟังก์ชันสาธารณะ
    pub structs: Vec<StructSignature>,     // โครงสร้างข้อมูล
}

pub struct FunctionSignature {
    pub name: String,              // ชื่อฟังก์ชัน
    pub is_entry: bool,            // เรียกจากภายนอกได้หรือไม่
    pub type_params: Vec<String>,  // type parameters
    pub parameters: Vec<ParameterInfo>, // parameters
    pub returns: Vec<String>,      // return types
    pub doc: Option<String>,       // เอกสารคำอธิบาย
}
```

## 🚀 การใช้งาน

### 1. การอัพโหลด Contract

```rust
use kanari_move_runtime::{
    BlockchainEngine,
    ContractDeployment,
    ContractMetadata,
};

// เตรียม metadata
let metadata = ContractMetadata::new(
    "MyToken".to_string(),
    "1.0.0".to_string(),
    "0x1".to_string(),
)
.with_description("Token contract".to_string())
.with_license("MIT".to_string())
.with_tags(vec!["token".to_string(), "defi".to_string()]);

// สร้าง deployment
let deployment = ContractDeployment::new(
    module_bytecode,       // compiled Move bytecode
    "my_token".to_string(), // module name
    "0x1",                 // publisher address
    metadata,
)?
.with_gas_limit(1_000_000)
.with_gas_price(1500);

// Deploy
let tx_hash = engine.deploy_contract(deployment)?;
println!("Deployed: {}", hex::encode(tx_hash));
```

### 2. การเรียกใช้ Contract Function

```rust
use kanari_move_runtime::ContractCall;

// สร้าง call
let call = ContractCall::new(
    "0x1",           // contract address
    "my_token",      // module name
    "mint",          // function name
    "0x2",           // caller address
)?
.with_gas_limit(200_000)
.with_gas_price(1500);

// เพิ่ม arguments (BCS-encoded)
let amount = bcs::to_bytes(&1000u64)?;
let call = call.with_arg(amount);

// Execute
let tx_hash = engine.call_contract(call)?;
```

### 3. การค้นหา Contract

```rust
// ค้นหาโดย address และ module name
if let Some(contract) = engine.get_contract("0x1", "my_token") {
    println!("Contract: {}", contract.metadata.name);
    println!("Version: {}", contract.metadata.version);
    println!("Functions: {}", contract.abi.functions.len());
}

// ดู contracts ทั้งหมดของ address
let contracts = engine.list_contracts_by_address("0x1");
for contract in contracts {
    println!("- {}: {}", contract.module_name, contract.metadata.description);
}

// ค้นหาตาม tag
let token_contracts = engine.search_contracts_by_tag("token");
println!("Found {} token contracts", token_contracts.len());

// ดู contracts ทั้งหมด
let all = engine.list_all_contracts();
println!("Total contracts: {}", all.len());

// นับจำนวน
let count = engine.get_contract_count();
```

## ⛽ Gas Costs

### Contract Operations

- **Contract Deployment**: 60,000 + (module_size × 10) + (metadata_size × 5) gas units
- **Contract Call**: 35,000 + (function_name_len × 100) gas units
- **Contract Query**: 1,000 gas units
- **Module Publish**: 50,000 + (module_size × 10) gas units
- **Function Execute**: 30,000 + (complexity × 1,000) gas units

### ตัวอย่างการคำนวณ

```rust
use kanari_move_runtime::GasOperation;

// Contract deployment (1KB module, 200B metadata)
let gas = GasOperation::ContractDeployment {
    module_size: 1024,
    metadata_size: 200,
};
println!("Gas needed: {} units", gas.gas_units());
// Output: 71,240 units

// Contract call
let gas = GasOperation::ContractCall {
    function_name_len: 8,
};
println!("Gas needed: {} units", gas.gas_units());
// Output: 35,800 units
```

## 🔍 API Reference

### BlockchainEngine Methods

#### `deploy_contract(deployment: ContractDeployment) -> Result<Vec<u8>>`

อัพโหลด contract และคืนค่า transaction hash

#### `call_contract(call: ContractCall) -> Result<Vec<u8>>`

เรียกใช้ฟังก์ชันใน contract และคืนค่า transaction hash

#### `get_contract(address: &str, module_name: &str) -> Option<ContractInfo>`

ดึงข้อมูล contract

#### `list_contracts_by_address(address: &str) -> Vec<ContractInfo>`

ดู contracts ทั้งหมดของ address

#### `list_all_contracts() -> Vec<ContractInfo>`

ดู contracts ทั้งหมดในระบบ

#### `search_contracts_by_tag(tag: &str) -> Vec<ContractInfo>`

ค้นหา contracts ตาม tag

#### `get_contract_count() -> usize`

นับจำนวน contracts ทั้งหมด

## 📚 ตัวอย่างเพิ่มเติม

### ตัวอย่าง: Token Contract

```move
// sources/my_token.move
module 0x1::my_token {
    use std::signer;
    use kanari_system::coin;
    
    struct MyToken has drop {}
    
    public entry fun initialize(admin: &signer) {
        let (treasury, metadata) = coin::create_currency(
            MyToken {},
            9,
            b"MTK",
            b"My Token",
            b"A test token",
            option::none(),
            admin
        );
        // Store treasury...
    }
    
    public entry fun mint(admin: &signer, to: address, amount: u64) {
        // Mint tokens...
    }
    
    public entry fun transfer(from: &signer, to: address, amount: u64) {
        // Transfer tokens...
    }
}
```

### Deploy และเรียกใช้

```rust
// 1. Compile Move code
// move-cli build --save-metadata

// 2. Read bytecode
let bytecode = std::fs::read("build/my_token/bytecode.mv")?;

// 3. Deploy
let metadata = ContractMetadata::new(
    "MyToken".to_string(),
    "1.0.0".to_string(),
    "0x1".to_string(),
);

let deployment = ContractDeployment::new(
    bytecode,
    "my_token".to_string(),
    "0x1",
    metadata,
)?;

let tx_hash = engine.deploy_contract(deployment)?;

// 4. Initialize
let call = ContractCall::new("0x1", "my_token", "initialize", "0x1")?;
engine.call_contract(call)?;

// 5. Mint tokens
let recipient = bcs::to_bytes(&AccountAddress::from_hex_literal("0x2")?)?;
let amount = bcs::to_bytes(&1000u64)?;

let call = ContractCall::new("0x1", "my_token", "mint", "0x1")?
    .with_arg(recipient)
    .with_arg(amount);
    
engine.call_contract(call)?;
```

## 🔐 Security

### Best Practices

1. **Verify Bytecode**: ตรวจสอบ bytecode ก่อน deploy
2. **Gas Limits**: ตั้ง gas limit ที่เหมาะสม
3. **Metadata**: ระบุ license และ source URL
4. **Testing**: ทดสอบบน testnet ก่อน
5. **Versioning**: ใช้ semantic versioning

### Signature Verification

```rust
use kanari_crypto::keys::{generate_keypair, CurveType};

// Generate keypair
let keypair = generate_keypair(CurveType::Ed25519)?;

// Sign transaction
let mut signed_tx = SignedTransaction::new(tx);
signed_tx.sign(&keypair.private_key, CurveType::Ed25519)?;

// Submit (engine จะ verify signature อัตโนมัติ)
engine.submit_transaction(signed_tx)?;
```

## 🧪 Testing

```bash
# Run tests
cd crates/kanari-move-runtime
cargo test

# Run contract demo
cargo run --example contract_demo

# Run blockchain demo
cargo run --example blockchain_demo
```

## 📖 เอกสารเพิ่มเติม

- [Move Language](../../third_party/move/README.md)
- [Kanari Types](../kanari-types/README.md)
- [Gas System](./src/gas.rs)
- [State Manager](./src/state.rs)

## 🤝 Contributing

เปิดรับ contributions! ดูรายละเอียดที่ [CONTRIBUTING.md](../../CONTRIBUTING.md)

## 📄 License

MIT License - see [LICENSE](../../LICENSE)
