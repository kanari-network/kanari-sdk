# Kanari Multi-Node Setup Guide

คู่มือการรัน Kanari blockchain แบบหลาย node ด้วย libp2p

## คุณสมบัติ

- ✅ P2P networking ด้วย libp2p
- ✅ Automatic peer discovery ด้วย mDNS (สำหรับ local network)
- ✅ Kademlia DHT สำหรับ peer discovery
- ✅ Gossipsub protocol สำหรับ message propagation
- ✅ Block และ transaction synchronization
- ✅ Configurable P2P และ RPC ports

## การติดตั้ง

1. Build project:

```bash
cargo build --release
```

1. ไฟล์ที่ถูกสร้าง:

```
target/release/kanari-node
```

## การรัน Multi-Node

### Node 1 (Bootstrap Node)

```bash
kanari-node start --p2p-port 19000 --rpc-port 19001 --data-dir C:\Users\Pukpuy\.kanari\kanari-db\node1
```

### Node 2

```bash
kanari-node start --p2p-port 19010 --rpc-port 19011 --data-dir C:\Users\Pukpuy\.kanari\kanari-db\node2
```

### Node 3

```bash
kanari-node start --p2p-port 19020 --rpc-port 19021 --data-dir C:\Users\Pukpuy\.kanari\kanari-db\node3
```

## การเชื่อมต่อ Nodes

### อัตโนมัติ (Local Network)

หาก nodes ทำงานในเครือข่ายเดียวกัน จะค้นหากันเองผ่าน mDNS โดยอัตโนมัติ

### Manual Bootstrap

หากต้องการเชื่อมต่อกับ node เฉพาะ:

```bash
kanari-node start --p2p-port 19010 --rpc-port 19011 \
  --data-dir C:\Users\Pukpuy\.kanari\kanari-db\node2 \
  --bootstrap /ip4/127.0.0.1/tcp/19000/p2p/<PEER_ID>
```

โดย `<PEER_ID>` คือ Peer ID ที่แสดงในตอน node แรกเริ่มทำงาน

## ตัวอย่างการใช้งาน

### 1. ดู Stats ของ blockchain

```bash
kanari-node stats
```

### 2. ดูข้อมูล Account

```bash
kanari-node account 0x1
```

### 3. ดูข้อมูล Block

```bash
kanari-node block 0
```

### 4. List wallets

```bash
kanari-node list-wallets
```

### 5. ดูตัวเลือกทั้งหมด

```bash
kanari-node start --help
```

**ตัวเลือกที่สำคัญ:**

- `--p2p-port <PORT>` - กำหนด port สำหรับ P2P networking (default: 19000)
- `--rpc-port <PORT>` - กำหนด port สำหรับ RPC server (default: 19001)
- `--data-dir <PATH>` - กำหนดตำแหน่งเก็บข้อมูล blockchain และ state
- `--bootstrap <MULTIADDR>` - เชื่อมต่อกับ bootstrap peer (สามารถระบุหลายครั้ง)

## โครงสร้าง P2P Network

```
┌─────────────────────────────────────────────┐
│           Kanari P2P Network                │
├─────────────────────────────────────────────┤
│                                             │
│  Node 1 (19000) ←→ Node 2 (19010)          │
│       ↑                 ↓                   │
│       └─────→ Node 3 (19020)               │
│                                             │
└─────────────────────────────────────────────┘
```

### Protocols ที่ใช้

1. **Gossipsub** - สำหรับ broadcast blocks และ transactions
   - Topic: `kanari/blocks`
   - Topic: `kanari/transactions`
   - Topic: `kanari/peers`

2. **mDNS** - Auto-discovery ใน local network

3. **Kademlia DHT** - Distributed peer discovery

4. **Noise Protocol** - Encrypted transport

5. **Yamux** - Stream multiplexing

## P2P Message Types

```rust
pub enum P2PMessage {
    NewTransaction(String),  // Transaction ใหม่
    NewBlock(String),        // Block ใหม่
    BlockRequest(u64),       // ขอ block ตาม height
    BlockResponse(String),   // ตอบกลับด้วย block data
    PeerInfo(PeerInfoMsg),   // ข้อมูล peer (height, peer_id)
}
```

## การ Sync Blocks

เมื่อ node ใหม่เข้าร่วม network:

1. Node รับ `PeerInfo` จาก peers อื่นๆ
2. เปรียบเทียบ height กับ local blockchain
3. ถ้าต่ำกว่า จะส่ง `BlockRequest` เพื่อขอ blocks ที่ขาดหายไป
4. รับ `BlockResponse` และ apply blocks ไปยัง local chain

## Port Configuration

| Service | Default Port |   การปรับแต่ง  |
|---------|--------------|--------------|
| P2P     | 19000        | `--p2p-port` |
| RPC     | 19001        | `--rpc-port` |

## Data Directory Configuration

แต่ละ node ควรมี data directory แยกกันเพื่อป้องกันข้อมูลทับกัน:

### Windows

```bash
--data-dir C:\Users\<Username>\.kanari\kanari-db\node1
--data-dir C:\Users\<Username>\.kanari\kanari-db\node2
--data-dir C:\Users\<Username>\.kanari\kanari-db\node3
```

### Linux/macOS

```bash
--data-dir ~/.kanari/kanari-db/node1
--data-dir ~/.kanari/kanari-db/node2
--data-dir ~/.kanari/kanari-db/node3
```

**หมายเหตุ:** ถ้าไม่ระบุ `--data-dir` ระบบจะใช้ default directory ซึ่งอาจทำให้ nodes ใช้ข้อมูลร่วมกัน

## ตัวอย่าง Advanced Setup

### การใช้ PowerShell Scripts (Windows)

ในโฟลเดอร์นี้มี PowerShell scripts สำหรับช่วยในการรัน multi-node:

#### 1. Setup และดูข้อมูล Configuration

```powershell
.\setup-multi-node.ps1
```

Script นี้จะ:

- สร้าง data directories สำหรับแต่ละ node
- แสดงข้อมูล configuration ของแต่ละ node
- แสดงคำสั่งสำหรับการรัน nodes

#### 2. รัน Node แต่ละตัว

```powershell
# Terminal 1
.\start-node.ps1 -NodeId 1

# Terminal 2
.\start-node.ps1 -NodeId 2

# Terminal 3
.\start-node.ps1 -NodeId 3
```

### การรัน 5 Nodes พร้อมกัน (Manual)

```bash
# Terminal 1
kanari-node start --p2p-port 19000 --rpc-port 19001 --data-dir C:\Users\Pukpuy\.kanari\kanari-db\node1

# Terminal 2
kanari-node start --p2p-port 19010 --rpc-port 19011 --data-dir C:\Users\Pukpuy\.kanari\kanari-db\node2

# Terminal 3
kanari-node start --p2p-port 19020 --rpc-port 19021 --data-dir C:\Users\Pukpuy\.kanari\kanari-db\node3

# Terminal 4
kanari-node start --p2p-port 19030 --rpc-port 19031 --data-dir C:\Users\Pukpuy\.kanari\kanari-db\node4

# Terminal 5
kanari-node start --p2p-port 19040 --rpc-port 19041 --data-dir C:\Users\Pukpuy\.kanari\kanari-db\node5
```

## การ Monitor Network

ดู logs เพื่อตรวจสอบ P2P events:

```
INFO kanari_node: Node Peer ID: 12D3KooW...
INFO kanari_node: P2P network initialized on port 19000
INFO kanari_node: Listening on /ip4/0.0.0.0/tcp/19000
INFO kanari_node: Discovered peer: 12D3KooW... at /ip4/...
INFO kanari_node: Connection established with 12D3KooW...
INFO kanari_node: Received transaction from network: 0x...
INFO kanari_node: Received new block #123 from network
```

## Troubleshooting

### ปัญหา: Nodes ไม่เจอกัน

**แก้ไข:**

1. ตรวจสอบว่าใช้ port ไม่ซ้ำกัน
2. ตรวจสอบ firewall settings
3. ลอง manual bootstrap ด้วย `--bootstrap`

### ปัญหา: Block sync ไม่ทำงาน

**แก้ไข:**

1. ตรวจสอบ logs หา errors
2. ตรวจสอบว่า blocks ถูก broadcast หรือไม่
3. Restart nodes เพื่อ re-sync

## สถาปัตยกรรม

```
┌──────────────────────────────────────────┐
│         Kanari Node                      │
├──────────────────────────────────────────┤
│                                          │
│  ┌────────────┐      ┌──────────────┐    │
│  │    RPC     │      │  P2P Network │    │
│  │  Server    │      │  (libp2p)    │    │
│  └──────┬─────┘      └──────┬───────┘    │
│         │                   │            │
│         └────┬──────────────┘            │
│              │                           │
│      ┌───────▼────────┐                  │
│      │ Blockchain     │                  │
│      │ Engine         │                  │
│      └───────┬────────┘                  │
│              │                           │
│      ┌───────▼────────┐                  │
│      │ Move Runtime   │                  │
│      │ + State        │                  │
│      └────────────────┘                  │
│                                          │
└──────────────────────────────────────────┘
```

## การพัฒนาต่อ

TODO items ที่สามารถพัฒนาต่อได้:

- [ ] Block validation logic
- [ ] Consensus mechanism (PoS, PoW, etc.)
- [ ] Persistent peer storage
- [ ] NAT traversal สำหรับ WAN
- [ ] Transaction pool management
- [ ] Fork resolution
- [ ] State sync optimization
- [ ] Metrics และ monitoring dashboard

## License

Apache-2.0
