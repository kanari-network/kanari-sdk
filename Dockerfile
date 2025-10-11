# Multi-stage Dockerfile for building the `kari` CLI/server from the kanari-sdk workspace.
#
# Stage 1: build with official Rust image (uses stable toolchain per rust-toolchain.toml)
# Stage 2: create a minimal runtime image with the built binary and CA certificates.

########################################
# Build stage
########################################
FROM rust:1.90.0 AS builder

# Create app directory
WORKDIR /usr/src/kanari-sdk

# Cache dependencies by copying manifests first
COPY Cargo.toml Cargo.lock ./
COPY rust-toolchain.toml ./

# Copy workspace Cargo manifests to leverage layer caching (only paths that exist)
COPY crates/kari/Cargo.toml crates/kari/Cargo.toml
COPY crates/command/Cargo.toml crates/command/Cargo.toml
COPY crates/common/Cargo.toml crates/common/Cargo.toml

# Copy the rest of the workspace
COPY . .

# Install system dependencies needed to build native crates (rocksdb, bindgen/libclang, etc.)
# These packages ensure `libclang` is available for `bindgen` and provide common compression libs
# used by RocksDB and other C native dependencies.
RUN apt-get update \
	&& apt-get install -y --no-install-recommends \
		build-essential \
		clang \
		libclang-dev \
		cmake \
		pkg-config \
		git \
		ca-certificates \
		libssl-dev \
		zlib1g-dev \
		libsnappy-dev \
		libbz2-dev \
		liblz4-dev \
		libzstd-dev \
	&& rm -rf /var/lib/apt/lists/*

# Build the release binary. We target the `kari` crate which provides the CLI/server.
RUN cargo build -p kari --release --locked

########################################
# Runtime stage
########################################
FROM debian:bullseye-slim

# Install CA certificates for TLS
RUN apt-get update \
	&& apt-get install -y --no-install-recommends ca-certificates \
	&& rm -rf /var/lib/apt/lists/*

# Create a non-root user to run the binary
RUN useradd -m -u 1000 kanari

WORKDIR /home/kanari

# Copy the binary from the builder image
COPY --from=builder /usr/src/kanari-sdk/target/release/kari /usr/local/bin/kari
RUN chown kanari:kanari /usr/local/bin/kari && chmod +x /usr/local/bin/kari

USER kanari

ENTRYPOINT ["/usr/local/bin/kari"]

# Default to printing help; override command and args at runtime, e.g.:
# docker run --rm kanari-image server
