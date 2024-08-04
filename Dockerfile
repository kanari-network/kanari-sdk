# Use the official Rust image as the base
FROM rust:1.82 as builder

# Create a new empty shell project
RUN USER=root cargo new --bin kanari-network
WORKDIR /kanari-network

# Copy your project's files into the Docker container
COPY ./Cargo.toml ./Cargo.toml
COPY ./Cargo.lock ./Cargo.lock
COPY ./src ./src
COPY ./consensus ./consensus
COPY ./p2p ./p2p
COPY ./crates ./crates
COPY ./kari ./kari
COPY ./kari-explorer ./kari-explorer
COPY ./move-execution ./move-execution
COPY ./node ./node

# Build your project
RUN cargo build --release

# Start a new stage to reduce the size of the final image
# by not including the build tools and intermediate build artifacts
FROM debian:buster-slim
COPY --from=builder /kanari-network/target/release/kanari-network /usr/local/bin/kanari-network

# Set the command to run your application
CMD ["kanari-network"]