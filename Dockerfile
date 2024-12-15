# Use the official Rust image as the base
FROM rust:1.82 as builder

# Create a new empty shell project
RUN USER=root cargo new --bin kanari-sdk
WORKDIR /kanari-sdk

# Copy your project's files into the Docker container
COPY Cargo.toml Cargo.lock ./
COPY consensus consensus
COPY crates crates
COPY move-execution move-execution


# Build your project
RUN cargo build --release

# Start a new stage to reduce the size of the final image
FROM debian:buster-slim
COPY --from=builder /kanari-sdk/target/release/kanari-sdk /usr/local/bin/kanari-sdk

# Set the command to run your application
CMD ["kanari-sdk"]
