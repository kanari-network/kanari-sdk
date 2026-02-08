# Stage 1: Planning
FROM lukemathwalker/cargo-chef:latest-rust-1.93.0-slim-bookworm AS chef
WORKDIR /usr/src/kanari-sdk

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Builder
FROM chef AS builder
COPY --from=planner /usr/src/kanari-sdk/recipe.json recipe.json

# Copy third_party dependencies because they are path-based but NOT workspace members
# cargo-chef needs them to compile the skeleton.
COPY third_party third_party

# Install build dependencies
RUN apt-get update && apt-get install -y \
    clang \
    llvm \
    librocksdb-dev \
    pkg-config \
    libssl-dev \
    cmake \
    && rm -rf /var/lib/apt/lists/*

# Build dependencies
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source and build
COPY . .
RUN cargo build --release --bin kanari-node --bin kanari

# Stage 3: Runtime
FROM debian:bookworm-slim AS runtime

# Install only necessary runtime shared libraries
# librocksdb-dev is replaced by the actual shared library if possible, 
# but for simplicity and compatibility, we'll use the slim package.
RUN apt-get update && apt-get install -y \
    librocksdb7.8 \
    libssl3 \
    ca-certificates \
    && apt-get clean \
    && rm -rf /var/lib/apt/lists/*

WORKDIR /usr/local/bin

# Copy binaries from builder
COPY --from=builder /usr/src/kanari-sdk/target/release/kanari-node .
COPY --from=builder /usr/src/kanari-sdk/target/release/kanari .

# Set environment variables
ENV RUST_LOG=info

# Expose ports
EXPOSE 19000 19001 19002

# Default command
CMD ["./kanari-node", "start"]
