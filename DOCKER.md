# Building and running kanari-sdk with Docker

This document shows how to build and run a Docker image that contains the `kari` binary built from this workspace.

Windows (PowerShell):

```powershell
# Build the docker image (from repository root)
docker build -t kanari-sdk:latest .

# Show help
docker run --rm kanari-sdk:latest --help

# -----------------------------
# Generate a wallet (interactive, recommended)
# -----------------------------
# Use -it so secure password prompts use a TTY. Mount a host folder to persist the keystore
# and blockchain DB. On Windows PowerShell, prefer single quotes for the -v argument.
docker run --rm -it -v 'D:\Work\kanari-sdk\data\.kari:/home/kanari/.kari' kanari-sdk:latest keytool generate

# -----------------------------
# Start the server (foreground, see logs)
# -----------------------------
# Map RPC (default 30030) and UI (example 8080). The node also listens for P2P on 51303.
docker run --rm -it -p 30030:30030 -p 8080:8080 -v 'D:\Work\kanari-sdk\data\.kari:/home/kanari/.kari' kanari-sdk:latest server start

# -----------------------------
# Start the server (detached/background)
# -----------------------------
# Use the same -v mount so the keystore and RocksDB under ~/.kari persist.
docker run -d --name kanari-node -p 30030:30030 -p 8080:8080 -p 51303:51303 -v 'D:\Work\kanari-sdk\data\.kari:/home/kanari/.kari' kanari-sdk:latest server start

# View logs for the detached container
docker logs -f kanari-node

# Stop and remove the detached container
docker stop kanari-node; docker rm kanari-node
```

Linux / macOS (bash):

```bash
docker build -t kanari-sdk:latest .

docker run --rm kanari-sdk:latest --help

# Interactive wallet generation (recommended)
docker run --rm -it -v "$HOME/.kari:/home/kanari/.kari" kanari-sdk:latest keytool generate

# Start server (foreground)
docker run --rm -it -p 30030:30030 -p 8080:8080 -v "$HOME/.kari:/home/kanari/.kari" kanari-sdk:latest server start

# Start server (detached)
docker run -d --name kanari-node -p 30030:30030 -p 8080:8080 -p 51303:51303 -v "$HOME/.kari:/home/kanari/.kari" kanari-sdk:latest server start

# Tail logs
docker logs -f kanari-node
```

Notes and tips:

- The Dockerfile uses a multi-stage build. The first stage uses the official Rust image to compile the `kari` crate in release mode. The second stage is a slim Debian runtime image that contains the compiled binary and CA certificates.
- To reduce rebuild times during development you can use `--target-dir` to place build artifacts on a mounted volume, or build outside Docker and use `COPY` to include the binary in the image.
- If your environment requires a specific Rust version, update `rust-toolchain.toml` or change the `FROM rust:1.` line in the Dockerfile.
- The image runs as a non-root user `kanari` for better security.

Important notes:

- Keystore and blockchain DB locations
  - The node stores keystore/config and blockchain data under the Kari app directory: `~/.kari` inside the container (path: `/home/kanari/.kari`). Mount the host folder to this path to persist wallets and the RocksDB database between container runs.

- TTY and `keytool generate`
  - `keytool generate` prompts for a password using a secure TTY-based prompt. Run it with `-it` (interactive TTY) so `read_password()` works. Piping only the mnemonic length or other inputs without providing the password will cause the secure password prompt to fail and the wallet save will be rejected (empty password not allowed).
  - If you must automate wallet creation (CI), consider one of:
    - Use an interactive approach with an attached TTY (not always available in CI).
    - Pass the password via environment variable or file (not currently supported by the CLI; this requires a code change).

- Detached containers and shutdown behavior
  - If you run the server detached (`-d`), ensure the process does not read stdin on startup (it shouldn't). Use `docker logs -f` to follow output and `docker stop` to gracefully stop the node.

- DNS discovery warnings
  - On first run, the node may attempt to resolve discovery hostnames (devnet/testnet/mainnet). If DNS fails inside your environment, the node will run standalone. This is normal in private or offline networks.

If you want, I can also:

- Add a small Docker Compose file to run `kari` together with other services (e.g., a database).
- Switch to a smaller runtime image (e.g., scratch or distroless) if static linking is possible.
