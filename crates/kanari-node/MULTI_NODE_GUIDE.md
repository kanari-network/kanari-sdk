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

### Node 1 (Bootstrap Node / Authority 0x1)

```bash
cargo run --bin kanari-node -- start --p2p-port 19000 --rpc-port 19001 --data-dir data/node1 --authority-id 0x1 --authorities 0x1,0x2,0x3,0x4,0x5
```

### Node 2 (Authority 0x2)

```bash
cargo run --bin kanari-node -- start --p2p-port 19010 --rpc-port 19011 --data-dir data/node2 --authority-id 0x2 --authorities 0x1,0x2,0x3,0x4,0x5
```

### Node 3 (Authority 0x3)

```bash
cargo run --bin kanari-node -- start --p2p-port 19020 --rpc-port 19021 --data-dir data/node3 --authority-id 0x3 --authorities 0x1,0x2,0x3,0x4,0x5
```

## การเชื่อมต่อ Nodes

### อัตโนมัติ (Local Network)

หาก nodes ทำงานในเครือข่ายเดียวกัน จะค้นหากันเองผ่าน mDNS โดยอัตโนมัติ

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
- `--rpc-host <HOST>` - กำหนด host/IP สำหรับ RPC (default: 0.0.0.0)
- `--data-dir <PATH>` - กำหนดตำแหน่งเก็บข้อมูล blockchain และ state
- `--relay-server` - เปิดใช้งาน relay server mode เพื่อช่วย nodes ที่อยู่หลัง NAT
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

6. **DCUtR** - Direct Connection Upgrade through Relay สำหรับ NAT traversal

7. **Identify** - Peer information exchange (protocol version, addresses)

8. **Ping** - Connection keep-alive และ latency measurement

9. **Relay** - Circuit relay protocol สำหรับ nodes ที่อยู่หลัง strict NAT

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

## NAT Traversal และ Hole Punching

Kanari รองรับการเชื่อมต่อระหว่าง nodes ที่อยู่หลัง NAT/firewall ผ่าน:

### DCUtR (Direct Connection Upgrade through Relay)

- **Hole punching** - สร้างการเชื่อมต่อโดยตรงระหว่าง nodes ที่อยู่หลัง NAT
- **Identify protocol** - แลกเปลี่ยนข้อมูล peer (protocol version, listen addresses)
- **Ping protocol** - Keep-alive connections และตรวจสอบ latency

### การทำงาน

1. Nodes ใช้ **Identify protocol** เพื่อแลกเปลี่ยนข้อมูลและ addresses
2. **Kademlia DHT** ช่วยในการ discover peers across networks
3. **DCUtR** พยายามสร้างการเชื่อมต่อโดยตรง (hole punching)
4. ถ้าสำเร็จ, nodes จะสื่อสารกันโดยตรงโดยไม่ต้องผ่าน relay

**หมายเหตุ:** สำหรับ strict NAT/symmetric NAT อาจต้องใช้ relay server เพิ่มเติม (ดูข้อมูลด้านล่าง)

### Relay Server Mode

สำหรับ nodes ที่อยู่หลัง strict NAT หรือ symmetric NAT ที่ hole punching ไม่สามารถทำงานได้ สามารถใช้ relay server mode:

```bash
kanari-node start --p2p-port 19000 --rpc-port 19001 --relay-server
```

**คุณสมบัติ:**

- รับ reservation requests จาก nodes ที่ต้องการใช้ relay
- สร้าง circuit relay ระหว่าง nodes
- ช่วยให้ nodes ที่อยู่หลัง NAT สื่อสารกันได้แม้ว่า hole punching จะล้มเหลว

**แนะนำ:** รัน relay server บน node ที่มี public IP หรืออยู่บน network ที่ accessible จากภายนอก

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

### การรัน 6 Nodes พร้อมกัน (Manual)

```bash
# Terminal 1 (Authority 0x1)
cargo run --bin kanari-node -- start --p2p-port 19000 --rpc-port 19001 --data-dir data/node1 --authority-id 0x1 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 2 (Authority 0x2)
cargo run --bin kanari-node -- start --p2p-port 19010 --rpc-port 19011 --data-dir data/node2 --authority-id 0x2 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 3 (Authority 0x3)
cargo run --bin kanari-node -- start --p2p-port 19020 --rpc-port 19021 --data-dir data/node3 --authority-id 0x3 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 4 (Authority 0x4)
cargo run --bin kanari-node -- start --p2p-port 19030 --rpc-port 19031 --data-dir data/node4 --authority-id 0x4 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 5 (Authority 0x5)
cargo run --bin kanari-node -- start --p2p-port 19040 --rpc-port 19041 --data-dir data/node5 --authority-id 0x5 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 6 (Authority 0x6)
cargo run --bin kanari-node -- start --p2p-port 19050 --rpc-port 19051 --data-dir data/node6 --authority-id 0x6 --authorities 0x1,0x2,0x3,0x4,0x5,0x6
```

## RPC Endpoints

- Local (loopback):
  - Node 1: `http://127.0.0.1:19001`
  - Node 2: `http://127.0.0.1:19011`
  - Node 3: `http://127.0.0.1:19021`

- LAN (reachable from other machines on your network):
  - Node 1: `http://<machine_ip>:19001`
  - Node 2: `http://<machine_ip>:19011`
  - Node 3: `http://<machine_ip>:19021`

To expose RPC to the LAN, start each node with either `--rpc-host 0.0.0.0` (bind all interfaces) or `--rpc-host <machine_ip>` (bind a single interface). Example:

```powershell
kanari-node start --p2p-port 19000 --rpc-port 19001 --rpc-host 0.0.0.0 --data-dir C:\Users\Pukpuy\.kanari\kanari-db\node1
```

Security note: binding RPC to all interfaces exposes the API to your local network — ensure your firewall and network policies allow or block access as intended.

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

### ✅ คุณสมบัติที่ทำงานแล้ว

- [x] **Block synchronization** - Nodes สามารถ sync blocks จากกันได้ผ่าน P2P
- [x] **State sync optimization** - Execute transactions จาก synced blocks เพื่อ rebuild state
  - Skip sequence validation สำหรับ synced transactions
  - ใช้ main Move runtime เพื่อให้ module bytecode persist ถูกต้อง
  - Apply changesets เพื่อ update accounts, balances, modules, objects
- [x] **Transaction pool management** - Pending transaction pool พร้อม broadcast ผ่าน P2P
- [x] **P2P message propagation** - Gossipsub protocol สำหรับ broadcast blocks/transactions
- [x] **Peer discovery** - mDNS (local) และ Kademlia DHT
- [x] **Block validation logic** - ตรวจสอบ block hash, prev_hash chain, timestamp, transaction integrity
- [x] **Transaction deduplication** - ป้องกัน double spending และ replay attacks ด้วย transaction hash tracking
- [x] **Persistent peer storage** - บันทึกและโหลด peer list จาก disk เพื่อ reconnect อัตโนมัติ
- [x] **Transaction signature verification** - Verify signatures สำหรับ synced และ committed transactions
  - Block เก็บ `SignedTransaction` แทน `Transaction`
  - Verify signatures ใน `Block::verify()` และ `sync_full_block_from_data()`
  - ป้องกัน malicious blocks จาก compromised nodes
- [x] **Merkle tree for transactions** - Transaction merkle root ใน block header สำหรับ light client verification
  - ใช้ SMT's Blake3 hash function เพื่อความสอดคล้องกับ state tree
  - ทุก block มี merkle root ที่คำนวณจาก transaction hashes
  - Block validation ตรวจสอบ merkle root integrity
  - RPC endpoint `kanari_getTransactionMerkleProof` สำหรับ proof generation
  - Support proof verification สำหรับ light clients
- [x] **NAT traversal** - DCUtR (Direct Connection Upgrade through Relay) protocol สำหรับ hole punching
  - Identify protocol สำหรับ peer information exchange
  - Ping protocol สำหรับ connection keep-alive
  - รองรับการเชื่อมต่อโดยตรงระหว่าง nodes ที่อยู่หลัง NAT
  - ใช้ร่วมกับ Kademlia DHT สำหรับ peer discovery
- [x] **Relay server mode** - รัน node เป็น relay server สำหรับ nodes ที่อยู่หลัง strict NAT
  - ใช้ `--relay-server` flag เพื่อเปิดใช้งาน relay server mode
  - รองรับ circuit relay และ reservation requests
  - ช่วย nodes ที่อยู่หลัง symmetric NAT ให้สื่อสารกันได้

## Merkle Tree Architecture

Kanari ใช้ **2 ประเภท** ของ Merkle trees:

### 1. Sparse Merkle Tree (SMT) - State Storage

- **ตำแหน่ง**: `crates/smt/`
- **จุดประสงค์**: Account state verification และ proofs
- **ใช้สำหรับ**: Account balances, modules, objects, state root
- **Storage**: Persistent ใน RocksDB

### 2. Transaction Merkle Tree - Block Verification  

- **ตำแหน่ง**: `crates/kanari-core/src/blockchain/merkle.rs`
- **จุดประสงค์**: Light client transaction verification
- **ใช้สำหรับ**: Block header merkle root, transaction inclusion proofs
- **Storage**: In-memory, คำนวณใหม่ต่อ block

ดู [DOCS/MERKLE_TREES.md](../../../DOCS/MERKLE_TREES.md) สำหรับรายละเอียดเพิ่มเติม

### 🚧 TODO items ที่สามารถพัฒนาต่อได้

- [ ] **Consensus mechanism** - PoS, PoW, หรือ BFT consensus
- [ ] **Fork resolution** - Logic สำหรับจัดการ chain forks และเลือก canonical chain
- [ ] **Metrics และ monitoring dashboard** - Real-time network statistics

## License

Apache-2.0
