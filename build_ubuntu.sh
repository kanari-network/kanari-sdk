#!/bin/bash
echo "================================================"
echo " Attention - Building Kanari Network from source code."
echo " This will request root permissions with sudo."
echo "================================================"

# Install Ubuntu dependencies

sudo apt-get update

sudo apt-get install -y \
	curl \
 	git-all \
  	cmake \
   	gcc \
    	libssl-dev \
     	pkg-config \
      	libclang-dev \
       	libpq-dev \
	build-essential \
	clang \
	llvm \
	make \
	tmux \
	xz-utils \
	ufw

# Install Rust nightly
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y --default-toolchain nightly

# Source the environment
source $HOME/.cargo/env

# Install Kari CLI
cargo install --locked kari
