# Use the official Rust image as the base
FROM rust:1.58 as builder

# Create a new empty shell project
RUN USER=root cargo new --bin rust-blockchain
WORKDIR /rust-blockchain

# Copy your project's files into the Docker container
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./src ./src
COPY ./consensus ./consensus
COPY ./p2p ./p2p

# Build your project
RUN cargo build --release

# Start a new stage to reduce the size of the final image
# by not including the build tools and intermediate build artifacts
FROM debian:buster-slim
COPY --from=builder /rust-blockchain/target/release/rust-blockchain /usr/local/bin/rust-blockchain

# Set the command to run your application
CMD ["rust-blockchain"]