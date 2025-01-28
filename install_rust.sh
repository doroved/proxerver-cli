#!/bin/bash

# Updating package lists
echo "--> Updating package lists..."
sudo apt update -y > /dev/null 2>&1
sudo apt upgrade -y > /dev/null 2>&1

# Installing necessary packages
echo "--> Installing build-essential, pkg-config, and libssl-dev..."
sudo apt install build-essential pkg-config libssl-dev -y > /dev/null 2>&1

# Installing Rust without requiring confirmation
echo "--> Installing Rust..."
curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y > /dev/null 2>&1

# Install cross
echo "--> Installing cross..."
cargo install cross > /dev/null 2>&1

# Install musl-tools
echo "--> Installing musl-tools..."
sudo apt install musl-tools -y > /dev/null 2>&1

# Adding musl target
echo "--> Adding musl target..."
rustup target add x86_64-unknown-linux-musl > /dev/null 2>&1
rustup target add aarch64-unknown-linux-musl > /dev/null 2>&1

# Installing docker
echo "--> Installing docker..."
curl -fsSL https://get.docker.com -o get-docker.sh > /dev/null 2>&1
sudo sh get-docker.sh > /dev/null 2>&1
rm get-docker.sh

# Adding user to docker group
echo "--> Adding user to docker group..."
sudo usermod -aG docker $USER > /dev/null 2>&1

echo "--> Installation completed!"