#!/bin/bash
echo "================================================"
echo " Attention - Building Kanari Network from source code."
echo " This will request root permissions with sudo."
echo "================================================"

# Install Ubuntu dependencies

sudo apt-get update

sudo apt-get install -y \
	build-essential \
	curl \
	clang \
	gcc \
	libssl-dev \
	llvm \
	make \
	pkg-config \
	tmux \
	xz-utils \
	ufw


# Install Rust

curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
source $HOME/.cargo/env

# Install snarkOS
# cargo clean
cargo install --locked --path .
