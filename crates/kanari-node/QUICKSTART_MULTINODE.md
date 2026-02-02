# Quick Start Guide - Multi-Node Setup

## วิธีการรันอย่างรวดเร็ว

### ขั้นตอนที่ 1: Build โปรเจค

```powershell
cd C:\Users\Pukpuy\Desktop\kanari-sdk
cargo build --release
```

### ขั้นตอนที่ 2: เตรียม Data Directories

```powershell
# รัน setup script
cd crates\kanari-node
.\setup-multi-node.ps1
```

### ขั้นตอนที่ 3: เปิด 3 Terminals และรัน Nodes

**Terminal 1 - Node 1:**

```powershell
cd C:\Users\Pukpuy\Desktop\kanari-sdk\crates\kanari-node
.\start-node.ps1 -NodeId 1
```

**Terminal 2 - Node 2:**

```powershell
cd C:\Users\Pukpuy\Desktop\kanari-sdk\crates\kanari-node
.\start-node.ps1 -NodeId 2
```

**Terminal 3 - Node 3:**

```powershell
cd C:\Users\Pukpuy\Desktop\kanari-sdk\crates\kanari-node
.\start-node.ps1 -NodeId 3
```

### ขั้นตอนที่ 4: ตรวจสอบว่า Nodes เชื่อมต่อกันแล้ว

ดูที่ logs ในแต่ละ terminal:

```
INFO kanari_node: Discovered peer: 12D3KooW... at /ip4/...
INFO kanari_node: Connection established with 12D3KooW...
```

## การรันแบบ Manual (ไม่ใช้ Scripts)

```powershell
# Terminal 1 - Node 1 (Auth: 0x1)
cargo run --bin kanari-node -- start --p2p-port 19000 --rpc-port 19001 --data-dir data/node1 --authority-id 0x1 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 2 - Node 2 (Auth: 0x2)
cargo run --bin kanari-node -- start --p2p-port 19010 --rpc-port 19011 --data-dir data/node2 --authority-id 0x2 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 3 - Node 3 (Auth: 0x3)
cargo run --bin kanari-node -- start --p2p-port 19020 --rpc-port 19021 --data-dir data/node3 --authority-id 0x3 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 4 - Node 4 (Auth: 0x4)
cargo run --bin kanari-node -- start --p2p-port 19030 --rpc-port 19031 --data-dir data/node4 --authority-id 0x4 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 5 - Node 5 (Auth: 0x5)
cargo run --bin kanari-node -- start --p2p-port 19040 --rpc-port 19041 --data-dir data/node5 --authority-id 0x5 --authorities 0x1,0x2,0x3,0x4,0x5,0x6

# Terminal 6 - Node 6 (Auth: 0x6)
cargo run --bin kanari-node -- start --p2p-port 19050 --rpc-port 19051 --data-dir data/node6 --authority-id 0x6 --authorities 0x1,0x2,0x3,0x4,0x5,0x6
```

## ตรวจสอบสถานะ Blockchain

เปิด terminal ใหม่และรัน:

```powershell
# ดู stats
kanari-node stats

# ดูข้อมูล account
kanari-node account 0x1

# ดูข้อมูล block
kanari-node block 0
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

## การทดสอบ P2P Network

1. ส่ง transaction ผ่าน RPC ของ Node 1
2. ตรวจสอบว่า transaction ปรากฏใน Node 2 และ Node 3
3. เมื่อมีการ produce block ที่ Node ใดก็ตาม blocks จะถูก sync ไปยัง nodes อื่นๆ

## หมายเหตุสำคัญ

⚠️ **แต่ละ node ต้องมี `--data-dir` แยกกัน** เพื่อป้องกันข้อมูลทับกัน

⚠️ **ต้องใช้ ports ที่แตกต่างกัน** สำหรับ `--p2p-port` และ `--rpc-port`

✅ Nodes จะค้นหากันเองโดยอัตโนมัติผ่าน mDNS (ใน local network)

✅ ข้อมูล blockchain และ state จะถูกเก็บแยกกันในแต่ละ data directory

## เอกสารเพิ่มเติม

ดูเอกสารฉบับเต็มที่: [MULTI_NODE_GUIDE.md](MULTI_NODE_GUIDE.md)
