#!/usr/bin/env bash

set -euo pipefail

SCRIPT_DIR=$(cd -- "$(dirname -- "${BASH_SOURCE[0]}")" && pwd)
REPO_ROOT=$(cd -- "${SCRIPT_DIR}/.." && pwd)
ZKSYNC_OS_DIR="${REPO_ROOT}/zksync-os"
ARTIFACTS_DIR="${REPO_ROOT}/artifacts"

# This function forms a metadata message to be included in the artifacts.
# It includes the git branch name (if any), commit hash, build timestamp, and whether the
# working directory has uncommitted changes for the `zksync-os` repository (e.g. submodule).
# It then writes it to a file named build_metadata.txt in the artifacts directory.
function generate_build_metadata() {
    local git_branch
    git_branch=$(git -C "${ZKSYNC_OS_DIR}" rev-parse --abbrev-ref HEAD || echo "N/A")
    local git_commit
    git_commit=$(git -C "${ZKSYNC_OS_DIR}" rev-parse HEAD)
    local build_timestamp
    build_timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    local dirty_flag
    if [[ -n $(git -C "${ZKSYNC_OS_DIR}" status --porcelain) ]]; then
        dirty_flag="(with uncommitted changes)"
    else
        dirty_flag="(clean)"
    fi

    cat > "${ARTIFACTS_DIR}/build_metadata.txt" << EOF
Git Branch: ${git_branch}
Git Commit: ${git_commit}
Build Timestamp (UTC): ${build_timestamp}
Repository Status: ${dirty_flag}
EOF
}

mkdir -p "${ARTIFACTS_DIR}"

# zksync-os remains the source of the executable we prove. The recursion
# verification keys are generated from ethereum_prover's current Airbender
# dependency so 80-bit and 100-bit security use fresh matching single-file VKs.
cargo run --manifest-path "${REPO_ROOT}/Cargo.toml" --release -p ethereum_prover -- \
    generate-verifier-artifacts \
    --output-dir "${ARTIFACTS_DIR}"

cp "${ZKSYNC_OS_DIR}/zksync_os/app.bin" "${ARTIFACTS_DIR}/"
cp "${ZKSYNC_OS_DIR}/zksync_os/app.text" "${ARTIFACTS_DIR}/"

generate_build_metadata
echo "Artifacts copied successfully."
