use std::path::PathBuf;
use std::sync::Once;

use alloy::rpc::types::{Block as RpcBlock, debug::ExecutionWitness};
use ethereum_prover::prover::types::EthBlockInput;

fn manifest_dir() -> PathBuf {
    std::env::var_os("CARGO_MANIFEST_DIR")
        .map(PathBuf::from)
        .filter(|path| path.join("test_fixtures").is_dir())
        .unwrap_or_else(|| PathBuf::from(env!("CARGO_MANIFEST_DIR")))
}

pub fn fixture_root() -> PathBuf {
    manifest_dir().join("test_fixtures")
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
    manifest_dir().join("../artifacts/app.bin")
}

pub fn load_fixture_input(fixture: &str) -> EthBlockInput {
    let block_path = fixture_block_path(fixture);
    let witness_path = fixture_witness_path(fixture);
    let block_json = std::fs::read_to_string(&block_path)
        .unwrap_or_else(|err| panic!("read fixture block {}: {err}", block_path.display()));
    let witness_json = std::fs::read_to_string(&witness_path)
        .unwrap_or_else(|err| panic!("read fixture witness {}: {err}", witness_path.display()));
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
