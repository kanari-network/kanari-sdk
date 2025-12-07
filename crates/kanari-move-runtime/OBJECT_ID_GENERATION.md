# Object ID Generation System

## ภาพรวม

ระบบ Object ID ใน Kanari Move Runtime ได้รับการปรับปรุงให้สามารถ generate ID ได้อัตโนมัติสำหรับทุกประเภทของ Move objects

## การทำงาน

### 1. Object Detection
ระบบจะตรวจจับ objects ที่ถูกสร้างขึ้นจาก Move VM execution โดยอัตโนมัติ:

```rust
// ตรวจจับจาก write-set changes ที่มี Op::New
for (addr, bytes) in new_resources {
    // สร้าง object entry
}
```

### 2. Object ID Generation

ระบบใช้ 2 วิธีในการสร้าง Object ID:

#### วิธีที่ 1: Extract จาก UID (มาตรฐาน Move)
สำหรับ objects ที่มี `UID` field (มาตรฐาน Move object model):
```rust
if bytes.len() >= 32 {
    // Extract ID จาก 32 bytes แรก (UID.addr)
    let id_bytes = &bytes[0..32];
    let obj_id = hex::encode(id_bytes);
}
```

#### วิธีที่ 2: Generate ด้วย Blake3 Hash
สำหรับ objects ที่ไม่มี UID หรือมี size < 32 bytes:
```rust
fn generate_object_id(
    owner: &AccountAddress,
    struct_tag: &StructTag,
    data: &[u8],
) -> String {
    // สร้าง unique input
    let mut input = Vec::new();
    input.extend_from_slice(owner.as_ref());           // owner address
    input.extend_from_slice(struct_tag.address.as_ref()); // module address
    input.extend_from_slice(struct_tag.module.as_str().as_bytes()); // module name
    input.extend_from_slice(struct_tag.name.as_str().as_bytes());   // struct name
    input.extend_from_slice(data);                     // object data
    
    // Hash เพื่อสร้าง ID ที่ unique
    let hash = hash_data_blake3(&input);
    hex::encode(&hash[0..32])
}
```

### 3. Object Storage

Object ที่ถูก detect จะถูกเก็บใน StateManager:

```rust
pub struct StateManager {
    /// Map of object id -> CreatedObject
    pub objects: HashMap<String, CreatedObject>,
    
    /// Per-account list of owned object ids
    pub owned_objects: HashMap<AccountAddress, Vec<String>>,
}
```

### 4. Object Structure

```rust
pub struct CreatedObject {
    pub id: String,              // Unique object ID (hex-encoded)
    pub owner: AccountAddress,   // Owner's address
    pub type_: String,          // Full type name (e.g., "0x1::coin::Coin<0x2::james::JAMES>")
    pub data: Vec<u8>,          // Serialized object data
    pub version: u64,           // Object version (0 for new objects)
}
```

## ตัวอย่างการใช้งาน

### สร้าง Object ใน Move

```move
module 0x123::nft {
    use std::object::{Self, UID};
    
    struct MyNFT has key {
        id: UID,
        name: vector<u8>,
        image_url: vector<u8>,
    }
    
    public fun mint_nft(name: vector<u8>, image_url: vector<u8>): MyNFT {
        MyNFT {
            id: object::new(),  // สร้าง UID ใหม่
            name,
            image_url,
        }
    }
}
```

### Query Objects ผ่าน RPC

```rust
// Get all objects owned by an account
let objects = state_manager.owned_objects.get(&owner_address);

// Get specific object by ID
let object = state_manager.objects.get(&object_id);

println!("Object ID: {}", object.id);
println!("Owner: {}", object.owner);
println!("Type: {}", object.type_);
```

## ประโยชน์

1. **Automatic ID Generation**: ไม่ต้องกำหนด ID เอง ระบบ generate ให้อัตโนมัติ
2. **Unique & Deterministic**: ID ที่สร้างมีความ unique และสร้างซ้ำได้ (deterministic)
3. **Universal Support**: รองรับทุกประเภท Move objects
4. **Owner Tracking**: ติดตาม ownership โดยอัตโนมัติ
5. **Query Capability**: สามารถ query objects ตาม owner หรือ ID

## Technical Details

### ID Format
- ความยาว: 64 hex characters (32 bytes)
- Encoding: Lowercase hexadecimal
- ตัวอย่าง: `"a1b2c3d4e5f6..."`

### Hash Algorithm
- ใช้ Blake3 (fast & secure)
- Input: owner + module_address + module_name + struct_name + data
- Output: 32-byte hash

### Performance
- การ generate ID ใช้เวลาน้อยมาก (nanoseconds)
- ไม่มีผลกระทบต่อ performance ของ Move VM
- รองรับการสร้าง objects หลายพันรายการต่อ transaction

## การทดสอบ

```bash
# Build และ test
cd crates/kanari-move-runtime
cargo test

# ตรวจสอบ object creation
cargo run --example object_creation_test
```

## ข้อจำกัดและข้อควรระวัง

1. Objects ต้องผ่าน Move VM execution เท่านั้น (ไม่สามารถสร้าง manually)
2. Object ID จะถูก generate ครั้งเดียวเมื่อสร้าง object (immutable)
3. การลบ object ไม่ได้ลบ ID ออกจากระบบ (เก็บ history)

## สรุป

ระบบ Object ID Generation ใหม่รองรับการสร้าง unique identifiers สำหรับทุกประเภท Move objects อย่างอัตโนมัติ โดยใช้ Blake3 hashing และ UID extraction ทำให้สามารถติดตามและ query objects ได้อย่างมีประสิทธิภาพ
