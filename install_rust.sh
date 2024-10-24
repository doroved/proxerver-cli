#!/bin/bash

# Updating package lists
echo "--> Updating package lists..."
sudo apt update

# Installing necessary packages
echo "--> Installing build-essential, pkg-config, and libssl-dev..."
sudo apt install build-essential pkg-config libssl-dev -y

# Installing Rust
echo "--> Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh

# Checking Ubuntu version
echo "--> Checking Ubuntu version..."
# cat /etc/os-release
lsb_release -a

# Checking GLIBC version
echo "--> Checking GLIBC version..."
ldd --version

# Checking libssl version
echo "--> Checking libssl version..."
ldconfig -p | grep libssl

# Setting environment variables for OpenSSL
echo "--> Setting environment variables for OpenSSL..."
export OPENSSL_LIB_DIR=/usr/lib/$(arch)-linux-gnu
export OPENSSL_INCLUDE_DIR=/usr/include/openssl

echo "--> Installation completed!"
echo "--> Setting up environment..."
echo "To activate changes, run the command: source \$HOME/.cargo/env"
echo "--> Checking rustc version: rustc --version"