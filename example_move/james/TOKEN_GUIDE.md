# JAMES Token - การใช้งาน

## สถาปัตยกรรมใหม่

ระบบ token ถูกออกแบบให้ **flexible** และ **ใช้งานผ่าน CLI** เท่านั้น:

1. **ไม่ auto mint** - เมื่อ publish module จะไม่มีการ mint token อัตโนมัติ
2. **ใช้ `init` function** - รันเมื่อ publish เพื่อสร้าง TreasuryCap
3. **TreasuryCap** ถูกส่งไปที่ publisher - ใช้สำหรับ mint ภายหลัง
4. **เรียก `mint` ผ่าน CLI** - ยืดหยุ่น, ควบคุมได้

## ขั้นตอนการใช้งาน

### 1. Publish Module

```bash
cd james
kanari move publish --sender <YOUR_ADDRESS> --password <PASSWORD>
```

**ผลลัพธ์:**

- Module `james::james` ถูก publish
- `init()` function รันอัตโนมัติ
- TreasuryCap<JAMES> ถูกสร้างและส่งไปที่ YOUR_ADDRESS
- **ยังไม่มี token ถูก mint**

### 2. Mint Token

หลัง publish แล้ว ใช้ CLI mint token:

```bash
kanari move call \
  --sender 0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea \
  --password 12345678 \
  --module "0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea::james" \
  --function mint \
  --args <TREASURY_CAP_OBJECT_ID> 1000000000 0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea
```

**Parameters:**

- `<TREASURY_CAP_OBJECT_ID>` - Object ID ของ TreasuryCap (ดูจาก getAccount)
- `1000000000` - จำนวน token ที่จะ mint (1 billion JAMES)
- `0x840512ff...` - Address ที่จะรับ token

### 3. ตรวจสอบ Balance

```bash
kanari account get --address 0x840512ff2c03135d82d55098f7461579cfe87f5c10c62718f818c0beeca138ea
```

**ผลลัพธ์:**

```json
{
  "address": "0x840512ff...",
  "balance": 999999900000,
  "sequence": 3,
  "modules": ["james"],
  "token_balances": {
    "0x840512ff...::james::JAMES": 1000000000
  }
}
```

### 4. Transfer Token

```bash
kanari move call \
  --sender 0x840512ff... \
  --password 12345678 \
  --module "0x840512ff...::james" \
  --function transfer \
  --args <COIN_OBJECT_ID> <RECIPIENT_ADDRESS>
```

## ข้อดีของวิธีใหม่

### ✅ Flexible

- ไม่ lock amount ตอน deploy
- Mint ได้หลายครั้ง
- ควบคุม supply ได้ตลอด

### ✅ Universal

- ใครก็ deploy token ของตัวเองได้
- ไม่ต้องแก้ code ทุกครั้ง
- Pattern เดียวกันสำหรับ token ทุกตัว

### ✅ Security

- เฉพาะคนที่มี TreasuryCap mint ได้
- TreasuryCap transfer ได้ (ถ้าต้องการเปลี่ยน owner)
- Burn ได้ (ลด supply)

## Function รายละเอียด

### `init(witness: JAMES, ctx: &mut TxContext)`

- **Auto-run** เมื่อ publish module
- สร้าง currency metadata
- Freeze metadata (ไม่สามารถแก้ไขได้)
- Transfer TreasuryCap ไปที่ publisher

### `mint(treasury_cap: &mut TreasuryCap<JAMES>, amount: u64, recipient: address, ctx: &mut TxContext)`

- **Public entry** - เรียกจาก CLI ได้
- ต้องมี TreasuryCap
- Mint coin และส่งไปที่ recipient
- **ตัวอย่าง:** Mint 1M JAMES = `1000000` (ไม่นับ decimals)

### `transfer(c: Coin<JAMES>, recipient: address)`

- Transfer coin object ไปที่ recipient
- ต้องมี Coin<JAMES> object

### `burn(treasury_cap: &mut TreasuryCap<JAMES>, coin: Coin<JAMES>)`

- Burn coin (ลด total supply)
- ต้องมี TreasuryCap

## ตัวอย่างการใช้งาน Complete Flow

```bash
# 1. Start node
cargo run -p kanari-node

# 2. Create account
kanari account create --password 12345678

# 3. Publish module
cd james
kanari move publish --sender 0x840512ff... --password 12345678

# 4. Get account info (เช็ค TreasuryCap object ID)
kanari account get --address 0x840512ff...

# 5. Mint tokens
kanari move call \
  --sender 0x840512ff... \
  --password 12345678 \
  --module "0x840512ff...::james" \
  --function mint \
  --args <TREASURY_CAP_ID> 1000000000 0x840512ff...

# 6. Check balance
kanari account get --address 0x840512ff...

# 7. Transfer (ถ้ามี recipient อื่น)
kanari move call \
  --sender 0x840512ff... \
  --password 12345678 \
  --module "0x840512ff...::james" \
  --function transfer \
  --args <COIN_OBJECT_ID> 0xcad2d7c...
```

## การ Deploy Token อื่น

ใครก็สามารถ copy pattern นี้ไปใช้ได้:

```move
module mytoken::mytoken {
    use kanari_system::coin;
    use kanari_system::coin::{Coin, TreasuryCap};
    use kanari_system::tx_context::TxContext;
    use kanari_system::transfer;
    use std::string;
    use std::ascii;
    use std::option;

    struct MYTOKEN has drop {}

    fun init(witness: MYTOKEN, ctx: &mut TxContext) {
        let (treasury_cap, metadata) = coin::create_currency(
            witness,
            6,  // decimals
            ascii::string(b"MYT"),
            string::utf8(b"My Token"),
            string::utf8(b"Description"),
            option::none(),
            ctx
        );
        transfer::public_freeze_object(metadata);
        transfer::public_transfer(treasury_cap, tx_context::sender(ctx));
    }

    public entry fun mint(
        treasury_cap: &mut TreasuryCap<MYTOKEN>,
        amount: u64,
        recipient: address,
        ctx: &mut TxContext
    ) {
        let coin = coin::mint(treasury_cap, amount, ctx);
        transfer::public_transfer(coin, recipient);
    }
}
```

## Notes

- **Object Storage**: Coin objects จะถูกเก็บใน object storage
- **Token Balances**: ระบบจะ sync จาก object storage อัตโนมัติ
- **Event Processing**: Events จาก Move VM จะถูก process เพื่ออัพเดท state
- **Gas Fees**: ทุก transaction ต้องจ่าย gas ใน KANARI

## Troubleshooting

### TreasuryCap หายไปไหน?

- Check `kanari account get --address <YOUR_ADDRESS>`
- ดูใน objects list

### Mint ไม่ได้

- ต้องมี TreasuryCap object
- ตรวจสอบว่า object ID ถูกต้อง
- ตรวจสอบว่า sender เป็น owner ของ TreasuryCap

### Balance ไม่อัพเดท

- ตรวจสอบว่า transaction success
- Check events ใน transaction receipt
- Restart node ถ้าจำเป็น
