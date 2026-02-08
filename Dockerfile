# Stage 1: Planning
FROM lukemathwalker/cargo-chef:latest-rust-1.93.0-slim-bookworm AS chef
WORKDIR /usr/src/kanari-sdk

FROM chef AS planner
COPY . .
RUN cargo chef prepare --recipe-path recipe.json

# Stage 2: Builder
FROM chef AS builder
COPY --from=planner /usr/src/kanari-sdk/recipe.json recipe.json

# Copy path dependencies (third_party) because they are required during 'cook'
COPY third_party third_party

# Install build dependencies
RUN apt-get update && apt-get install -y \
    clang llvm librocksdb-dev pkg-config libssl-dev cmake \
    && rm -rf /var/lib/apt/lists/*

# Build dependencies
RUN cargo chef cook --release --recipe-path recipe.json

# Copy source and build
COPY . .
RUN cargo build --release --bin kanari-node --bin kanari

# Stage 3: Runtime
# Using distroless for the smallest possible secure image
# 'cc' variant includes libstdc++ which is needed for RocksDB
FROM gcr.io/distroless/cc-debian12 AS runtime

WORKDIR /usr/local/bin

# Copy binaries from builder
COPY --from=builder /usr/src/kanari-sdk/target/release/kanari-node .
COPY --from=builder /usr/src/kanari-sdk/target/release/kanari .

# Copy required shared libraries from builder
# We need RocksDB, SSL, and other dependencies of the compiled binaries
COPY --from=builder /usr/lib/x86_64-linux-gnu/librocksdb.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libssl.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libcrypto.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libz.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libbz2.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libsnappy.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/liblz4.so* /usr/lib/x86_64-linux-gnu/
COPY --from=builder /usr/lib/x86_64-linux-gnu/libzstd.so* /usr/lib/x86_64-linux-gnu/

ENV RUST_LOG=info
EXPOSE 19000 19001 19002

# Use the full path for the binary in distroless
ENTRYPOINT ["/usr/local/bin/kanari-node"]
CMD ["start"]
