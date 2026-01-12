#!/bin/env bash

set -e

echo "Rebuilding RISC-V binary"
cd zksync-os/zksync_os
./dump_bin.sh --type pectra

echo "Generating setup for RISC-V verifier"
cd ../
RUST_MIN_STACK=267108864 cargo test --release -p eth_runner -- generate_setup_and_layout_for_final_proof --ignored
echo "Rebuilding artifacts completed."
