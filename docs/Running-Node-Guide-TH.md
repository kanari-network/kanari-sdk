# คู่มือการใช้งาน Kanari Blockchain Node

คู่มือนี้จะอธิบายวิธีการติดตั้ง ใช้งาน และจัดการ Kanari blockchain node รวมถึงการตั้งค่าต่างๆ การเชื่อมต่อเครือข่าย และการติดตามสถานะ

## สารบัญ
1. [ข้อกำหนดเบื้องต้น](#ข้อกำหนดเบื้องต้น)
2. [การติดตั้ง Node](#การติดตั้ง-node)
3. [การใช้งานพื้นฐาน](#การใช้งานพื้นฐาน)
4. [การตั้งค่าคอนฟิก](#การตั้งค่าคอนฟิก)
5. [โหมดเครือข่าย](#โหมดเครือข่าย)
6. [การตั้งค่าความปลอดภัย](#การตั้งค่าความปลอดภัย)
7. [การติดตามสถานะ Node](#การติดตามสถานะ-node)
8. [การแก้ไขปัญหา](#การแก้ไขปัญหา)
9. [การตั้งค่าขั้นสูง](#การตั้งค่าขั้นสูง)

## ข้อกำหนดเบื้องต้น

ก่อนเริ่มใช้งาน Kanari blockchain node ต้องมีสิ่งต่อไปนี้:

- ติดตั้ง Kanari SDK และ CLI tools แล้ว
- สร้าง wallet อย่างน้อย 1 อัน (จำเป็นสำหรับการทำงานของ node)
- พื้นที่ดิสก์เพียงพอสำหรับข้อมูล blockchain (~10GB แนะนำ)
- เปิดพอร์ตเครือข่ายหากใช้งานแบบ network mode (ค่าเริ่มต้น: 51303 สำหรับ P2P, 30030 สำหรับ RPC)

## การติดตั้ง Node

### การติดตั้ง Kanari CLI

```bash
# ดาวน์โหลดและติดตั้ง Kanari CLI
curl -sSL https://get.kanari.site | bash

# ตรวจสอบการติดตั้ง
kari --version
```

### การสร้าง Wallet

จำเป็นต้องมี wallet เพื่อใช้งาน node หากยังไม่มี:

```bash
# สร้าง wallet ใหม่
kari keytool generate

# ดูรายการ wallet ที่มี
kari keytool list

# ตรวจสอบว่ามี wallet อยู่หรือไม่
kari keytool check
```

### การเริ่มต้นการตั้งค่า

```bash
# เริ่มต้นการตั้งค่า kanari.yaml
kari server init

# ตรวจสอบไฟล์การตั้งค่า
cat ~/.kari/kanari_config/kanari.yaml
```

## การใช้งานพื้นฐาน

### การเริ่มต้น Node

เริ่มต้น node ด้วยการตั้งค่าเริ่มต้น:

```bash
kari server start
```

การทำงานของคำสั่งนี้:
- เริ่มต้น blockchain node พร้อม RPC service บนพอร์ต 30030
- ใช้ wallet เริ่มต้นสำหรับการทำงาน blockchain
- เปิดใช้งาน TLS encryption ตามค่าเริ่มต้น
- อนุญาตการเชื่อมต่อจากภายนอก (network mode)

### การหยุด Node

หยุดการทำงานของ node โดยกด `Enter` ในเทอร์มินัลที่ node กำลังทำงาน

Node จะบันทึกสถานะ blockchain อัตโนมัติเมื่อปิด

## การตั้งค่าคอนฟิก

Kanari node รองรับการตั้งค่าหลายแบบ:

### การเปลี่ยน RPC Port

เปลี่ยนพอร์ต API (ค่าเริ่มต้น: 30030):

```bash
kari server start --port 30031
```

### การเลือก Wallet

กำหนด wallet ที่ต้องการใช้:

```bash
kari server start --wallet 0x1234567890abcdef
```

### การเชื่อมต่อกับ Peers

เชื่อมต่อกับ node อื่นๆ:

```bash
kari server start --peer node1.kanari.network:51303 --peer 192.168.1.100:51303
```

### การรวมคำสั่ง

```bash
kari server start --port 30031 --wallet myWallet --peer 192.168.1.100:51303 --localhost false --use-tls
```

## โหมดเครือข่าย

### Localhost Only Mode

ใช้งานแบบ localhost เท่านั้น (ไม่รับการเชื่อมต่อจากภายนอก):

```bash
kari server start --localhost
```

เหมาะสำหรับ:
- การพัฒนาและทดสอบ
- การใช้งาน node หลายตัวในเครื่องเดียว
- การเพิ่มความปลอดภัยในสภาพแวดล้อมส่วนตัว

### Network Mode

ใช้งานแบบเครือข่าย (ค่าเริ่มต้น):

```bash
kari server start --localhost false
```

อนุญาตให้ node ของคุณ:
- รับการเชื่อมต่อจาก node อื่น
- เข้าร่วมในระบบค้นหา peer
- ส่งต่อ blocks และ transactions ผ่านเครือข่าย

## การตั้งค่าความปลอดภัย

### TLS Encryption

ค่าเริ่มต้น TLS encryption จะเปิดใช้งาน หากต้องการปิด:

```bash
kari server start --use-tls=false
```

## การติดตามสถานะ Node

ขณะที่ node ทำงาน คุณจะเห็นการอัปเดตแบบ real-time ในคอนโซล:

- การสร้างหรือรับ blocks ใหม่
- การเชื่อมต่อและตัดการเชื่อมต่อ peers
- การประมวลผล transactions
- ข้อความแสดงข้อผิดพลาดและคำเตือน

ตัวอย่างผลลัพธ์:
```
Using network configuration:
  Port: 30030
  Localhost only: false
  Use TLS: false
  Peers: ["192.168.1.100:51303"]

Node network information:
  RPC API:   192.168.1.50:30030 (HTTP)
  P2P:       192.168.1.50:51303

Node will connect to the following peers:
  - 192.168.1.100:51303

Block status will be shown below. Press Enter to stop the node.
Block #1 created successfully
Block #2 received from peer
Transaction processed: 0xabc123...
```

## การแก้ไขปัญหา

### ปัญหาที่พบบ่อย

1. **ไม่มี Wallet**
```bash
No wallet found!
Please create a wallet first using:
kari keytool generate
```

**วิธีแก้**: สร้าง wallet ใหม่ตามคำแนะนำ

2. **พอร์ตถูกใช้งานแล้ว**
```bash
Failed to start RPC server: Address already in use
```

**วิธีแก้**: เปลี่ยนพอร์ต หรือหยุดโปรแกรมที่ใช้พอร์ตนั้น
```bash
kari server start --port 30031
```

3. **ไม่สามารถเชื่อมต่อ Peer**
```bash
Warning: No peers configured. Running in standalone mode.
```

**วิธีแก้**: ตรวจสอบ IP และพอร์ตของ peer หรือใช้ localhost mode

4. **Chain ID ว่างเปล่าหรือไม่ถูกต้อง**
```yaml
chain_id: ''
```

**สาเหตุ**: อาจเกิดจาก `CHAIN_ID` constant ในโค้ดที่เป็นค่าว่าง

**วิธีแก้**: 
```bash
# วิธีที่ 1: ลบไฟล์การตั้งค่าและสร้างใหม่
rm ~/.kari/kanari_config/kanari.yaml
kari server init

# วิธีที่ 2: แก้ไขด้วยตนเอง
nano ~/.kari/kanari_config/kanari.yaml
# เปลี่ยน chain_id: '' เป็น chain_id: 'kari-local-001'

# วิธีที่ 3: บังคับให้ระบบแก้ไขอัตโนมัติ
kari server start --port 30030

# วิธีที่ 4: เปลี่ยน environment แล้วกลับมา
kari env switch dev
kari env switch local
```

**หมายเหตุ**: หลังจากแก้ไขแล้ว ระบบจะแสดงข้อความ "Updated chain_id to: kari-local-001"

5. **Environment ไม่ตรงกับ Chain ID**
```bash
# ตรวจสอบ environment ปัจจุบัน
kari env list

# เปลี่ยน environment ให้ตรงกับการใช้งาน
kari env switch local  # สำหรับ development
kari env switch test   # สำหรับ testing
```

### การตรวจสอบสถานะ

```bash
# ตรวจสอบการตั้งค่า
cat ~/.kari/kanari_config/kanari.yaml

# ตรวจสอบ chain_id ปัจจุบัน
grep "chain_id:" ~/.kari/kanari_config/kanari.yaml

# ตรวจสอบ wallet
kari keytool list

# ตรวจสอบพอร์ตที่ใช้งาน
netstat -an | grep :30030
```

## การตั้งค่าขั้นสูง

### การตั้งค่าไฟล์ kanari.yaml

ไฟล์การตั้งค่าหลักจะอยู่ที่ `~/.kari/kanari_config/kanari.yaml`:

```yaml
keystore_path: "C:\\Users\\YourName\\.kari\\kanari_config\\kanari.keystore"
active_address: "0xd00bdd88b00cb017950243f92afa3c1d0a0b75f22f5f4f738aebb58133235599"
envs:
  - alias: "local"
    rpc: "http://127.0.0.1:30030"
    ws: "ws://127.0.0.1:30031"
  - alias: "dev"
    rpc: "https://dev-seed.kanari.site"
    ws: "wss://dev-seed.kanari.site/websocket"
  - alias: "test"
    rpc: "https://test-seed.kanari.site"
    ws: "wss://test-seed.kanari.site/websocket"
  - alias: "main"
    rpc: "https://main-seed.kanari.site"
    ws: "wss://main-seed.kanari.site/websocket"
active_env: "local"
localhost_only: false
use_tls: true
rpc_port: 30030
chain_id: "kari-local-001"  # ต้องไม่เป็นค่าว่าง
peers: []
```

**หมายเหตุสำคัญ**: 
- `chain_id` ต้องไม่เป็นค่าว่าง หากเป็นค่าว่างให้ลบไฟล์และสร้างใหม่
- สำหรับ environment ต่างๆ จะมี chain_id ดังนี้:
  - `local`: `kari-local-001`
  - `dev`: `kari-dev-001`
  - `test`: `kari-testnet-001`
  - `main`: `kari-mainnet-001`

### การเปลี่ยนแปลงการตั้งค่า

```bash
# ดู environment ทั้งหมด
kari env list

# เปลี่ยน environment
kari env switch test
kari env switch dev
kari env switch main

# เพิ่ม environment ใหม่
kari env add local_test http://127.0.0.1:30035

# ลบ environment (ไม่สามารถลบ built-in environments: local, dev, test, main)
kari env remove local_test

# เปลี่ยน active wallet
kari keytool select

# ดูรายการ wallet ทั้งหมด
kari keytool list
```

### การจัดการ Environment

Environment คือการตั้งค่าเครือข่ายที่แตกต่างกัน:

```bash
# ดู environment ปัจจุบันและทั้งหมด
kari env list

# ตัวอย่างผลลัพธ์:
# AVAILABLE ENVIRONMENTS
# NAME            RPC URL                                  STATUS
# local           http://127.0.0.1:30030                  ACTIVE
# dev             https://dev-seed.kanari.site
# test            https://test-seed.kanari.site
# main            https://main-seed.kanari.site

# เปลี่ยนไปใช้ testnet
kari env switch test

# เปลี่ยนไปใช้ mainnet
kari env switch main

# กลับมาใช้ local development
kari env switch local
```

### การจัดการ Wallet

```bash
# ดู wallet ทั้งหมด
kari keytool list

# เลือก wallet ที่ต้องการใช้
kari keytool select

# สร้าง wallet ใหม่
kari keytool generate

# import wallet จาก private key
kari keytool privatekey

# import wallet จาก seed phrase
kari keytool import

# ตรวจสอบยอดเงิน
kari keytool balance

# โอนเงิน
kari keytool transfer
```

### การจัดการ Mnemonic และ Session

```bash
# บันทึก mnemonic phrase
kari keytool mnemonic save

# โหลด mnemonic phrase
kari keytool mnemonic load

# ตรวจสอบสถานะ mnemonic
kari keytool mnemonic status

# ลบ mnemonic (ระวัง!)
kari keytool mnemonic remove

# จัดการ session keys
kari keytool session set api_key your_api_key
kari keytool session get api_key
kari keytool session remove api_key
kari keytool session clear
```

## การอัปเดต

```bash
# อัปเดต Kanari CLI
curl -sSL https://get.kanari.site | bash

# ตรวจสอบเวอร์ชันใหม่
kari --version

# อัปเดตการตั้งค่า (หากจำเป็น)
kari server init --update
```

สำหรับข้อมูลเพิ่มเติมและการสนับสนุน โปรดเยี่ยมชม [Kanari Documentation](https://docs.kanari.site) หรือติดต่อ [Community Discord](https://discord.gg/kanari)
