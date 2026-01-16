use std::path::{Path, PathBuf};
use std::sync::Once;

use alloy::rpc::types::{Block as RpcBlock, debug::ExecutionWitness};
use ethereum_prover::prover::types::EthBlockInput;

pub fn fixture_root() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("test_fixtures")
}

pub fn fixture_block_path(fixture: &str) -> PathBuf {
    fixture_root()
        .join("blocks")
        .join(fixture)
        .join("block.json")
}

pub fn fixture_witness_path(fixture: &str) -> PathBuf {
    fixture_root()
        .join("blocks")
        .join(fixture)
        .join("execution_witness.json")
}

pub fn app_bin_path() -> PathBuf {
    Path::new(env!("CARGO_MANIFEST_DIR")).join("../artifacts/app.bin")
}

pub fn load_fixture_input(fixture: &str) -> EthBlockInput {
    let block_json =
        std::fs::read_to_string(fixture_block_path(fixture)).expect("read fixture block");
    let witness_json =
        std::fs::read_to_string(fixture_witness_path(fixture)).expect("read fixture witness");
    let block: RpcBlock = serde_json::from_str(&block_json).expect("parse fixture block");
    let witness: ExecutionWitness =
        serde_json::from_str(&witness_json).expect("parse fixture witness");
    EthBlockInput::new(block, witness)
}

pub fn init_tracing() {
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let filter = tracing_subscriber::EnvFilter::builder()
            .with_default_directive("info".parse().unwrap())
            .from_env()
            .unwrap_or_else(|_| tracing_subscriber::EnvFilter::new("info"));
        tracing_subscriber::fmt().with_env_filter(filter).init();
    });
}
