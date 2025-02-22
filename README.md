Kanari SDK

# Kanari SDK Architecture

```mermaid
flowchart TB
  %% Application Layer
  subgraph "Application Layer"
    A1["CLI Tools"]
    A2["Web Explorer A"]
    A3["Web Explorer B"]
    A4["RPC API"]
  end

  %% Framework Layer
  subgraph "Framework Layer"
    F1["Kanari SDK"]
    F2["Move Standard Library"]
    F3["System SDK"]
  end

  %% Core Layer
  subgraph "Core Layer"
    %% Blockchain Sub-layer
    subgraph "Blockchain"
      C1["K2 Core"]
      C2["TX Management"]
      C3["State Management"]
    end
    %% Runtime Sub-layer
    subgraph "Runtime"
      R1["Move VM"]
      R2["Move Compiler"]
    end
    %% Consensus Sub-layer
    subgraph "Consensus"
      CS1["PoW Engine"]
      CS2["PoS Engine"]
    end
  end

  %% Network Layer
  subgraph "Network Layer"
    N1["P2P Network"]
    N2["RPC Network"]
  end

  %% Storage Layer
  subgraph "Storage Layer"
    S1["Wallet Store"]
    S2["File Store"]
    S3["State DB"]
  end

  %% Legend
  subgraph "Legend"
    L1["Application Layer (Light Blue)"]
    L2["Framework Layer (Light Green)"]
    L3["Core Layer (Light Pink)"]
    L4["Network Layer (Khaki)"]
    L5["Storage Layer (Light Grey)"]
  end

  %% Inter-layer Connections
  %% Application -> Framework
  A1 --> F1
  A2 --> F1
  A3 --> F1
  A4 --> F1

  %% Framework -> Core
  F1 --> C1
  F1 --> R1

  %% Internal Core - Blockchain, Runtime, Consensus
  C1 --> C2
  C1 --> C3
  R1 --> R2
  %% Bidirectional interactions between Blockchain and Runtime
  C1 <--> R1
  %% Bidirectional interactions between Blockchain and Consensus engines
  C1 <--> CS1
  C1 <--> CS2

  %% Core -> Network
  C1 --> N1
  C1 --> N2

  %% Network internal bidirectional exchange
  N1 <--> N2

  %% Core -> Storage
  C1 --> S3
  C3 --> S3
  S1 --> S3
  S2 --> S3

  %% Class Definitions for color coding
  class A1,A2,A3,A4 application
  class F1,F2,F3 framework
  class C1,C2,C3,R1,R2,CS1,CS2 core
  class N1,N2 network
  class S1,S2,S3 storage

  classDef application fill:#ADD8E6,stroke:#333,stroke-width:2px;
  classDef framework fill:#90EE90,stroke:#333,stroke-width:2px;
  classDef core fill:#FFB6C1,stroke:#333,stroke-width:2px;
  classDef network fill:#F0E68C,stroke:#333,stroke-width:2px;
  classDef storage fill:#D3D3D3,stroke:#333,stroke-width:2px;

  %% Click Events from Component Mapping
  click A1 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/crates/command"
  click A2 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/web/kari-explorer"
  click A3 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/web/tool_mata"
  click A4 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/crates/rpc/rpc-api"
  click F1 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/framework/kanari-framework"
  click F2 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/framework/move-stdlib"
  click F3 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/framework/kanari-system"
  click C1 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/crates/core/k2"
  click R1 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/crates/kari-move"
  click R2 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/crates/kari-move"
  click CS1 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/consensus/pow"
  click CS2 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/consensus/pos"
  click N1 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/crates/core/p2p"
  click N2 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/crates/core/network"
  click S1 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/crates/core/wallet"
  click S2 "https://github.com/kanari-network/kanari-sdk/tree/kanari-sdk/monaos/mona-storage"
```

### 1. Setup Environment
- Ensure you have Ubuntu or a similar Linux distribution.
- Install necessary dependencies like `build-essential`, `curl`, `clang`, `gcc`, `libssl-dev`, `llvm`, `make`, `pkg-config`, `tmux`, `xz-utils`, `ufw` using the provided `build_ubuntu.sh` script.

### 2. Install Rust
- Rust is required for building and running the Kari chain. Use the command provided in the `build_ubuntu.sh` script to install Rust and its package manager, Cargo.

### 3. Clone the Kari Chain Repository
- Obtain the source code for Kari chain. This might involve cloning a Git repository or downloading source code from a specific location.

### 4. Build the Project
- Navigate to the project directory.
- Run `cargo build --release` to compile the project. This command compiles the project in release mode, optimizing the binary for performance.

### 5. Run the Node
- After building, you can start a Kari chain node using `cargo run --release`. This command runs the compiled project.
- Depending on the project's specifics, you might need to add additional flags or configuration files to successfully start a node.

### 6. Interact with the Kari Chain
- Use the provided tools and documentation to interact with the Kari chain. This could involve sending transactions, mining blocks, or querying the blockchain state.

### 7. Update and Maintain
- Regularly update your local repository with the latest changes from the main Kari chain source.
- Rebuild the project as needed to ensure you're running the latest version.

### Usage Example
```shell
# Update and install dependencies
./build_ubuntu.sh

# Clone the Kari chain repo 
git clone  https://github.com/kanari-network/kanari-sdk.git
cd kanari-network

# Initialize and update submodules
git submodule init
git submodule update --init --recursive

# Build the project
cargo build --release
```

### Environment Setup
```shell

# Generate wallet
cargo run --release --bin kari keytool generate

# Configure and start node
# Interactive prompts will ask for:
# 1. Node type (enter 1 for validator)
# 2. RPC port (default: 3031)
# 3. Network domain (default: devnet.kari.network)
cargo run --release --bin kari start

# Start Kari node
# Windows
cargo run --release --bin kari start

# Linux/MacOS
cargo run --release --bin kari start
```

### Kari CLI Install

#### Prerequisites for Windows
- Install [Visual Studio Build Tools](https://visualstudio.microsoft.com/visual-cpp-build-tools/)
- Install [Rust](https://www.rust-lang.org/tools/install)
- Install MinGW via chocolatey:
```shell
choco install mingw
```
```shell
cargo install --locked --git https://github.com/kanari-network/kanari-sdk.git --branch main kari
```
## Star History

[![Star History Chart](https://api.star-history.com/svg?repos=kanari-network/kanari-sdk&type=Timeline)](https://star-history.com/#kanari-network/kanari-sdk&Timeline)
