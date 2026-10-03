# Airbender Ethereum prover

The v3 service records Ethereum execution witnesses with zksync-os
`av_integrate_v3`, proves them with `airbender-host::GpuProver`, and produces
security-100 unified proofs through Airbender's v3 `prover_pipeline`. It can
follow the chain tip, process a block range, archive proofs, and submit them to
EthProofs through a durable outbox.

The [TypeScript/WASM verifier](proof_verifier_js/README.md), package version
**1.2.0**, verifies gzip `EPROOF01` v2 proofs against a trusted `EVKEY001` v2 key.
Version 1/security-80 proofs need the old package and its matching artifacts.

## Checkout and build

The four repositories must be siblings because Cargo uses relative path dependencies:

```text
cluster/
  ethereum-prover/
  zksync-os/             # av_integrate_v3-compatible revision
  airbender-platform/    # v3 airbender-host and cargo-airbender
  zksync-airbender/      # v3 prover_pipeline and trusted FSV binaries
```

Use the compatible commits in [.github/stack-revisions.env](.github/stack-revisions.env).
The old nested `zksync-os` submodule is not used. CI fetches those three revisions
next to the prover checkout; the pinned commits must be published upstream before
remote CI can use them.

Build with `nightly-2026-08-09`, CUDA 13.3.1, and `RUST_MIN_STACK=1073741824`.
Install the sibling `cargo-airbender` as described in
[scripts/README.md](scripts/README.md). The
[container build](docker/ethereum-prover/README.md) prepares a reproducible guest
on the host Docker daemon, then builds the service and complete runtime bundle:

```sh
scripts/build_docker.sh -t ethereum-prover:v3
```

For a native build, install the guest tools described in [scripts/README.md](scripts/README.md),
then run `scripts/rebuild_artifacts.sh`. It rebuilds `eth-stf-fusaka` reproducibly, ships the
trusted FSV `.bin`/`.text` files, generates the v2 key, and records source commits
and SHA-256 hashes. See [artifacts/README.md](artifacts/README.md) for tracked and
generated files. Always set `FSV_DIR` when relocating the service.

## Run and verify

From `ethereum_prover/`, after building artifacts:

```sh
. ../artifacts/recursion.env
export FSV_DIR="$(realpath ../artifacts/fsv)"
export RUST_MIN_STACK=1073741824
# Set eth_prover_rpc_url to a node with debug_executionWitness enabled.
../target/release/ethereum_prover --config configs/local_debug.yaml block 26078503
../target/release/ethereum_prover verify /path/to/proof.bin.gz \
  --key ../artifacts/recursion_unified_v3_security_100.vk.bin
```

`local_debug.yaml` executes witnesses on CPU and keeps submission off. Use a
`gpu_prove` config for proofs. The [service README](ethereum_prover/README.md)
documents `run [--start N [--end M]]`, `block`, `prove`, `verify`, artifact
creation, configuration, persistent storage, and outbox recovery.

## License

[MIT](LICENSE-MIT) or [Apache 2.0](LICENSE-APACHE).
