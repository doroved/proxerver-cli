#!/bin/bash

# Check architecture
arch=$(uname -m)
if [[ "$arch" != "x86_64" && "$arch" != "aarch64" ]]; then
    echo "Error: Unsupported architecture $arch. Exiting script."
    exit 1
fi

# Function to add PATH to the configuration file
add_to_path() {
    local rc_file=$1
    if ! grep -q "export PATH=.*proxerver/bin" "$rc_file"; then
        echo "# Proxerver" >> "$rc_file"
        echo "export PATH=\$PATH:~/.proxerver/bin" >> "$rc_file"
        source "$rc_file"
        echo "Updated $rc_file"
    else
        echo "Path already added in $rc_file"
    fi
}

# Fetch the latest release from GitHub
curl "https://api.github.com/repos/doroved/proxerver/releases/latest" |
    grep '"tag_name":' |
    sed -E 's/.*"([^"]+)".*/\1/' |
    xargs -I {} curl -OL "https://github.com/doroved/proxerver/releases/download/"\{\}"/proxerver.${arch}.tar.gz"

# Create directory for installation
mkdir -p ~/.proxerver/bin

# Extract and move the files
tar -xzvf ./proxerver.${arch}.tar.gz && \
    rm -rf ./proxerver.${arch}.tar.gz && \
    rm ./._proxerver && \
    mv ./proxerver ~/.proxerver/bin

# Check for errors in the previous commands
if [ $? -ne 0 ]; then
    echo "Error. Exiting now."
    exit
fi

# Add to PATH
export PATH=$PATH:~/.proxerver/bin

# Check for .bashrc and .zshrc and append PATH export if they exist
if [ -f ~/.bashrc ]; then
    add_to_path ~/.bashrc
fi

if [ -f ~/.zshrc ]; then
    add_to_path ~/.zshrc
fi

# Success message with version
proxerver_version=$(proxerver -V)
echo ""
echo "Successfully installed $proxerver_version"

# Run the proxerver help command
proxerver --help
echo ""
echo "!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!!";
echo "Please copy and paste this command into the terminal and press Enter:"
echo "export PATH=\$PATH:~/.proxerver/bin"