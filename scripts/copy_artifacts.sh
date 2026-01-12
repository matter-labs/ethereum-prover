#!/bin/env bash

set -e

# This function forms a metadata message to be included in the artifacts.
# It includes the git branch name (if any), commit hash, build timestamp, and whether the
# working directory has uncommitted changes for the `zksync-os` repository (e.g. submodule).
# It then writes it to a file named build_metadata.txt in the artifacts directory.
function generate_build_metadata() {
    local git_branch
    git_branch=$(git -C ../zksync-os rev-parse --abbrev-ref HEAD || echo "N/A")
    local git_commit
    git_commit=$(git -C ../zksync-os rev-parse HEAD)
    local build_timestamp
    build_timestamp=$(date -u +"%Y-%m-%dT%H:%M:%SZ")
    local dirty_flag
    if [[ -n $(git -C ../zksync-os status --porcelain) ]]; then
        dirty_flag="(with uncommitted changes)"
    else
        dirty_flag="(clean)"
    fi

    cat > ../artifacts/build_metadata.txt << EOF
Git Branch: ${git_branch}
Git Commit: ${git_commit}
Build Timestamp (UTC): ${build_timestamp}
Repository Status: ${dirty_flag}
EOF
}


cp ../zksync-os/tests/instances/eth_runner/recursion_unified_setup.bin ../artifacts/
cp ../zksync-os/tests/instances/eth_runner/recursion_unified_layouts.bin ../artifacts/
cp ../zksync-os/zksync_os/app.bin ../artifacts/

generate_build_metadata
echo "Artifacts copied successfully."
