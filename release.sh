#!/bin/bash

# Extract project name and version from Cargo.toml
project_name=$(grep '^name' Cargo.toml | sed 's/name = "\(.*\)"/\1/' | tr -d '[:space:]')
version=$(grep '^version' Cargo.toml | sed 's/version = "\(.*\)"/\1/' | tr -d '[:space:]')

# Define architectures for Linux
# architectures=("x86_64-unknown-linux-musl" "aarch64-unknown-linux-musl")
architectures=("x86_64-unknown-linux-musl")

# Build for each architecture
for arch in "${architectures[@]}"; do
    # Extract architecture for naming
    short_arch=$(echo $arch | sed 's/-unknown-linux-musl//')

    # Determine the appropriate architecture for the orb command
    if [ "$short_arch" = "x86_64" ]; then
        orb_arch="amd64"
    elif [ "$short_arch" = "aarch64" ]; then
        orb_arch="arm64"
    else
        echo "Unsupported architecture: $short_arch"
        exit 1
    fi

    # https://docs.orbstack.dev/machines/commands#orb
    orb -m ubuntu-24.04-$orb_arch cross build --release --target=$arch
    orbctl stop ubuntu-24.04-$orb_arch

    # Move the binary to the release directory with a new name
    mkdir -p ./target/release/v${version}
    mv ./target/$arch/release/$project_name ./target/release/v${version}/${project_name}.${short_arch}
done

# Change to the release directory
cd ./target/release/v${version} || exit

# Create tar.gz and delete the original binaries
for arch in "${architectures[@]}"; do
    short_arch=$(echo $arch | sed 's/-unknown-linux-musl//')
    binary_name="${project_name}.${short_arch}"
    mv ${binary_name} ${project_name}
    tar -czf "${binary_name}.tar.gz" --no-xattr "${project_name}"
    rm "${project_name}"
done
