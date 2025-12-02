# เอกสารการเรียก RPC ของ Kanari (ภาษาไทย)

เอกสารฉบับนี้อธิบายการเรียก JSON-RPC ไปยัง RPC engineของ Kanari ทั้งรูปแบบคำขอ/คำตอบ เมธอดที่รองรับ โครงสร้างข้อมูลสำคัญ และตัวอย่างการใช้งานทั้งแบบ `curl` และผ่าน `RpcClient` (Rust).

**URL เริ่มต้น**: RPC จะรันบน HTTP endpoint (เช่น `http://127.0.0.1:3000/` หรือ `http://127.0.0.1:3000/rpc`).

**รูปแบบคำขอ (JSON-RPC 2.0)**

คำขอทั่วไปเป็น JSON ตามมาตรฐาน JSON-RPC 2.0:

```json
{
  "jsonrpc": "2.0",
  "method": "kanari_getAccount",
  "params": "0x...",
  "id": 1
}
```

ฟิลด์สำคัญ:

- `jsonrpc`: เวอร์ชัน (`"2.0"`).
- `method`: ชื่อเมธอด เช่น `kanari_getAccount` (ดูรายการเมธอดด้านล่าง).
- `params`: พารามิเตอร์เมธอด (อาจเป็นค่าหรือตัวแปร JSON object ขึ้นกับเมธอด).
- `id`: เลขไอดีคำขอ (ใช้จับคู่คำตอบ).

**รูปแบบคำตอบ (RpcResponse)**

คำตอบจะมีรูปแบบ:

```json
{
  "jsonrpc": "2.0",
  "result": { /* ข้อมูลตามเมธอด */ },
  "id": 1
}
```

กรณีเกิดข้อผิดพลาด จะมีฟิลด์ `error` แทน `result`.

**เมธอดที่รองรับ (ชื่อเมธอด)**

- `kanari_getAccount` — ดึง `AccountInfo` ของที่อยู่หนึ่ง.
- `kanari_getBalance` — ดึงยอดคงเหลือ (u64) ของที่อยู่.
- `kanari_getBlock` — ดึง `BlockInfo` ตามความสูงบล็อก (height).
- `kanari_getBlockHeight` — ดึงความสูงบล็อกปัจจุบัน (u64).
- `kanari_getStats` — ดึง `BlockchainStats`.
- `kanari_submitTransaction` — ส่ง `SignedTransactionData` เพื่อประมวลผล.
- `kanari_publishModule` — เผยแพร่โมดูล Move (PublishModuleRequest).
- `kanari_callFunction` — เรียกใช้ฟังก์ชัน Move (CallFunctionRequest).
- `kanari_getContract` — ดึงข้อมูลคอนแทรค.
- `kanari_listContracts` — รายการคอนแทรคทั้งหมด.

ชื่อเมธอดเหล่านี้ถูกนิยามไว้ที่ `crates/kanari-rpc-api/src/lib.rs` (โมดูล `methods`).

**โครงสร้างข้อมูลสำคัญ (JSON / Rust mapping)**

- `SignedTransactionData` (ตัวอย่าง JSON):

```json
{
  "sender": "0xabcdef...",
  "recipient": "0x1234...",        // optional
  "amount": 1000,                    // optional
  "gas_limit": 100000,
  "gas_price": 1,
  "sequence_number": 0,
  "signature": null                  // หรือ base64/array ของ bytes
}
```

- `PublishModuleRequest` (ตัวอย่าง):

```json
{
  "sender": "0x...",
  "module_bytes": [ /* u8 array */ ],
  "module_name": "MyModule",
  "gas_limit": 200000,
  "gas_price": 1,
  "signature": null
}
```

- `CallFunctionRequest` (ตัวอย่าง):

```json
{
  "sender": "0x...",
  "package": "0xPackageAddress",
  "module": "ModuleName",
  "function": "fun_name",
  "type_args": ["u64"],
  "args": [[/* bytes for arg1 */],[/* bytes for arg2 */]],
  "gas_limit": 100000,
  "gas_price": 1,
  "signature": null
}
```

หมายเหตุ: รูปแบบ `args` เป็น `Vec<Vec<u8>>` ในโค้ด Rust — ต้องส่งเป็นอาร์เรย์ของไบต์ใน JSON.

**ตัวอย่างการเรียกด้วย curl**

- ดึงข้อมูลบัญชี:

```bash
curl -X POST http://127.0.0.1:3000/ -H "Content-Type: application/json" -d '
{
  "jsonrpc": "2.0",
  "method": "kanari_getAccount",
  "params": "0xbeea29083fee79171d91c39cc257a6ba71c6f1adb7789ec2dbbd79622d9dde42",
  "id": 1
}'
```

- ส่งธุรกรรม (ตัวอย่างง่าย):

```bash
curl -X POST http://127.0.0.1:3000/rpc -H "Content-Type: application/json" -d '
{
  "jsonrpc": "2.0",
  "method": "kanari_submitTransaction",
  "params": {
    "sender": "0xSENDER...",
    "recipient": "0xRECIPIENT...",
    "amount": 1000,
    "gas_limit": 100000,
    "gas_price": 1,
    "sequence_number": 0,
    "signature": null
  },
  "id": 5
}'
```

- เผยแพร่โมดูล (ตัวอย่าง):

```bash
curl -X POST http://127.0.0.1:3000/ -H "Content-Type: application/json" -d '
{
  "jsonrpc": "2.0",
  "method": "kanari_publishModule",
  "params": {
    "sender": "0xSENDER...",
    "module_bytes": [0,97, ... ],
    "module_name": "MyModule",
    "gas_limit": 200000,
    "gas_price": 1,
    "signature": null
  },
  "id": 7
}'
```

**ตัวอย่างการใช้งาน Rust (`RpcClient`)**

โค้ดตัวอย่างการใช้ `RpcClient` ที่อยู่ใน `crates/kanari-rpc-client/src/lib.rs`:

```rust
use kanari_rpc_client::RpcClient;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    let client = RpcClient::new("http://127.0.0.1:3000");

    // ดึงยอดบัญชี
    let account = client.get_account("0xbeea...").await?;
    println!("Account: {:?}", account);

    // ดึงความสูงบล็อก
    let height = client.get_block_height().await?;
    println!("Block height: {}", height);

    Ok(())
}
```

และตัวอย่างการส่งธุรกรรม (เรียก `submit_transaction`):

```rust
use kanari_rpc_client::RpcClient;
use kanari_rpc_api::SignedTransactionData;

let client = RpcClient::new("http://127.0.0.1:3000");
let tx = SignedTransactionData {
    sender: "0xSENDER...".to_string(),
    recipient: Some("0xRECIPIENT...".to_string()),
    amount: Some(1000),
    gas_limit: 100000,
    gas_price: 1,
    sequence_number: 0,
    signature: None,
};

let status = client.submit_transaction(tx).await?;
println!("Tx status: {:?}", status);
```

**การรันเซิร์ฟเวอร์ (คร่าว ๆ)**

ในโค้ดเซิร์ฟเวอร์ (`crates/kanari-rpc-server/src/lib.rs`) มีฟังก์ชัน `create_router` และ `start_server`:

- เรียก `start_server(engine, "127.0.0.1:3000").await` เพื่อเริ่มให้บริการ HTTP JSON-RPC.

**ข้อควรระวัง / Best practices**

- ที่อยู่ (`sender`, `recipient`, `package`) ควรเป็นรูปแบบ hex ที่ถูกต้อง (เช่น `0x...`).
- `signature` ต้องอยู่ในรูปแบบที่ engine คาดหวัง (byte array) — ระวังการ encode/decode (JSON base64 หรือ array ของตัวเลข).
- `args` สำหรับ `call_function` ต้องเป็นอาร์เรย์ไบต์ของแต่ละอาร์กิวเมนต์ ตามที่โมดูล Move คาดหวัง.
- ตั้งค่า `gas_limit` และ `gas_price` ให้เหมาะสมกับการเรียกที่ต้องการ.

ถ้าต้องการ เพิ่มตัวอย่าง end-to-end (เช่น การเซ็น transaction ด้วยคีย์จริง) บอกผมได้ ผมจะเพิ่มขั้นตอนการเซ็นและตัวอย่างการแปลง signature เป็นรูปแบบที่ RPC ยอมรับ.

---
ไฟล์นี้ถูกสร้างจากโค้ดใน `crates/kanari-rpc-api`, `crates/kanari-rpc-client`, และ `crates/kanari-rpc-server` เพื่อช่วยให้การเรียก RPC ชัดเจนขึ้น
