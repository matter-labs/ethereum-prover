#!/bin/bash

# This script is stolen from https://github.com/matter-labs/zksync-airbender-prover/blob/main/scripts/ubuntu_setup.sh

# Exit immediately if a command exits with a non-zero status.
set -e

echo "Starting the ZKsync development machine setup..."

# --- 1. Install Rust ---
echo "Installing Rust..."
if ! command -v rustc &> /dev/null
then
    curl --proto '=https' --tlsv1.2 -sSf https://sh.rustup.rs | sh -s -- -y
    source "$HOME/.cargo/env"
    echo "Rust installed successfully."
else
    echo "Rust is already installed."
fi

# --- 2. Install Development Packages ---
echo "Installing development packages (build-essential, libssl-dev, etc.)..."
sudo apt-get update
sudo apt-get install -y build-essential libssl-dev pkg-config clang cmake

# --- 3. Install Foundry ---
echo "Installing Foundry..."
if ! command -v foundryup &> /dev/null
then
    # This command downloads the 'foundryup' installer.
    curl -L https://foundry.paradigm.xyz | bash
    # Now, run 'foundryup' using its full path to install Foundry itself.
    # This step creates the necessary directory structure, including the 'env' file.
    "$HOME/.foundry/bin/foundryup"
    export PATH="$PATH:/home/evl/.foundry/bin"
    echo "Foundry installed successfully."
else
    echo "Foundry is already installed, updating..."
    "$HOME/.foundry/bin/foundryup"
fi


# --- 4. Install CUDA ---
echo "Installing CUDA Toolkit..."
# Check if nvidia-smi is available to prevent re-installation
if ! command -v nvidia-smi &> /dev/null
then
    wget https://developer.download.nvidia.com/compute/cuda/repos/ubuntu2404/x86_64/cuda-keyring_1.1-1_all.deb
    sudo dpkg -i cuda-keyring_1.1-1_all.deb
    sudo apt-get update
    sudo apt-get install -y cuda-toolkit-12-9
    sudo apt-get install -y nvidia-open
    rm cuda-keyring_1.1-1_all.deb

    echo "Adding CUDA environment variables to ~/.bashrc..."
    {
        echo ''
        echo '# CUDA Environment Variables'
        echo 'export CUDA_HOME=/usr/local/cuda'
        echo 'export LD_LIBRARY_PATH=$LD_LIBRARY_PATH:/usr/local/cuda/lib64:/usr/local/cuda/extras/CUPTI/lib64'
        echo 'export PATH=$PATH:$CUDA_HOME/bin'
    } >> "$HOME/.bashrc"
    echo "CUDA installed and configured."
else
    echo "NVIDIA driver/CUDA appears to be already installed. Skipping installation."
fi

echo "Verifying NVIDIA driver status..."
nvidia-smi

# Export CUDA variables for the current script session. This is more reliable than sourcing .bashrc.
echo "Exporting CUDA variables for this session..."
export CUDA_HOME=/usr/local/cuda
export LD_LIBRARY_PATH=${LD_LIBRARY_PATH}:/usr/local/cuda/lib64:/usr/local/cuda/extras/CUPTI/lib64
export PATH=${PATH}:${CUDA_HOME}/bin

# --- 5. Compile era-bellman-cuda ---
echo "Cloning and compiling era-bellman-cuda..."
if [ ! -d "era-bellman-cuda" ]; then
    git clone https://github.com/matter-labs/era-bellman-cuda.git
else
    echo "era-bellman-cuda repository already exists. Skipping clone."
fi
# Now cmake will find the CUDA compiler (nvcc) via the updated PATH
cmake -Bera-bellman-cuda/build -Sera-bellman-cuda/ -DCMAKE_BUILD_TYPE=Release
cmake --build era-bellman-cuda/build/

echo "Adding BELLMAN_CUDA_DIR to ~/.bashrc..."
BELLMAN_DIR="$(pwd)/era-bellman-cuda"
# Check if the variable is already set
if ! grep -q "BELLMAN_CUDA_DIR" "$HOME/.bashrc"; then
    {
        echo ''
        echo '# Bellman CUDA Directory'
        echo "export BELLMAN_CUDA_DIR=${BELLMAN_DIR}"
    } >> "$HOME/.bashrc"
    echo "BELLMAN_CUDA_DIR configured."
else
    echo "BELLMAN_CUDA_DIR is already set in ~/.bashrc."
fi

# Export the variable for the current script session
export BELLMAN_CUDA_DIR=${BELLMAN_DIR}

# --- 6. Download CRS File ---
echo "Downloading CRS file..."
CRS_FILE="zksync-airbender-prover/crs/setup_compact.key"
if [ ! -f "$CRS_FILE" ]; then
    # Ensure the directory exists
    mkdir -p zksync-airbender-prover/crs
    curl https://storage.googleapis.com/matterlabs-setup-keys-us/setup-keys/setup_compact.key --output $CRS_FILE
    echo "CRS file downloaded successfully."
else
    echo "CRS file already exists. Skipping download."
fi

echo ""
echo "------------------------------------------------"
echo "Setup complete!"
echo "Please run 'source ~/.bashrc' or restart your terminal to apply all changes."
echo "------------------------------------------------"

