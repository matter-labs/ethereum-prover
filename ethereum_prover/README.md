# ethereum_prover

## Purpose

`ethereum_prover` is a pipeline-style binary that ingests Ethereum blocks and produces witnesses or GPU proofs, with optional submission to EthProofs.

## Running

```sh
# Run a single block
RUST_MIN_STACK=267108864 cargo run --release -- --config configs/local_debug.yaml block 24073997

# Run continuously
RUST_MIN_STACK=267108864 cargo run --release -- --config configs/local_debug.yaml run
```

In a realistic scenario, to run the binary you need to:
1. create `.env` file and set required configs there OR set environment variables directly (see below)
2. choose one of the templates in the `configs` folder, edit it to match your preferences (see below)
3. run the binary in the `run` mode.

Additionally, note that for EthProofs only the `gpu_prove` mode is relevant.
`cpu_witness` mode is there only for debugging purposes, and it has a (very basic) automated debugger that would attempt
to understand which transaction cause issues in terms of failure (it does so by comparing local execution results against
transaction receipts fetched from L1).

### Build Notes

The project unconditionally builds the GPU prover, because keeping it behind the feature flag would complicate the development.
To build the project without CUDA drivers installed, set `ZKSYNC_USE_CUDA_STUBS` environment variable to `true`.

`RUST_MIN_STACK` is required since the code might have compile issue with default value.

## Configuration

There are three layers of configuration:

1) CLI arguments (mode + execution target)
- `--config <path>`: path to a YAML config file
- `run`: continuous block stream from RPC
- `block <number>`: process a single block (debug/fixture-style)

2) YAML config (shared “safe” arguments)
YAML files use the `eth_prover` root key, e.g.
```yaml
eth_prover:
  mode: "cpu_witness"
  cache_policy: "on_failure"
  block_mod: 1
  prover_id: 0
  on_failure: "exit"
```

See examples at [configs](./configs/)

3) Environment / `.env` (sensitive data)
Environment variables use the `eth_prover_` prefix, as shown in [`.env.example`](./.env.example).

### Config options

All options below can be set in YAML under `eth_prover:` or via environment variables:

- `app_bin_path` (env: `eth_prover_app_bin_path`)
- `mode` (env: `eth_prover_mode`) — `cpu_witness` or `gpu_prove`
- `cache_policy` (env: `eth_prover_cache_policy`) — `off`, `on_failure`, `always`
- `ethproofs_submission` (env: `eth_prover_ethproofs_submission`) — `off`, `staging`, `prod`
- `block_mod` (env: `eth_prover_block_mod`)
- `prover_id` (env: `eth_prover_prover_id`)
- `on_failure` (env: `eth_prover_on_failure`) — `exit` or `continue`
- `rpc_url` (env: `eth_prover_rpc_url`) — sensitive
- `ethproofs_token` (env: `eth_prover_ethproofs_token`) — sensitive
- `ethproofs_cluster_id` (env: `eth_prover_ethproofs_cluster_id`) — sensitive
- `sentry_dsn` (env: `eth_prover_sentry_dsn`) — sensitive, enables error reporting
- `prometheus_port` (env: `eth_prover_prometheus_port`) — enables Prometheus exporter

Reusable configs live in `ethereum_prover/configs/`:
- `ethproofs_prod.yaml`: production EthProofs submission defaults
- `ethproofs_staging.yaml`: staging EthProofs submission defaults
- `local_debug.yaml`: local debug defaults (CPU witness, single-block friendly)

## Testing

- Use `cargo nextest run -p ethereum_prover` for fast, reliable test runs.
- GPU tests are opt-in: `RUN_GPU_TESTS=1 cargo nextest run -p ethereum_prover --test gpu_prover_fixture`
- Prefer unit tests for new behavior; add integration tests in `ethereum_prover/tests/` only when needed.

## License

[MIT](../LICENSE-MIT) or [Apache 2.0](../LICENSE-APACHE) at your option.
