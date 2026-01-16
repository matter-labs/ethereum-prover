//! Important: each top-level file in `tests/` is compiled as a separate crate.
//! This might explode the compile time for tests, so avoid adding new files here.
//! Wherever possible, prefer using test vectors within a single test, or creating
//! unit tests.
//! This module is for heavy integration tests only.

mod common;

use ethereum_prover::prover::cpu_witness::CpuWitnessGenerator;
use ethereum_prover::prover::gpu_prover::Prover;
use ethereum_prover::prover::oracle::build_oracle;

macro_rules! require_gpu_tests {
    () => {
        if std::env::var("RUN_GPU_TESTS").ok().as_deref() != Some("1") {
            eprintln!("Skipping GPU test. Set RUN_GPU_TESTS=1 to enable.");
            return;
        }
    };
}

#[tokio::test]
async fn cpu_witness_from_fixture_block() {
    common::init_tracing();
    let input = common::load_fixture_input("24073997");
    let oracle = build_oracle(input.clone()).expect("build oracle");
    let generator = CpuWitnessGenerator::new(common::app_bin_path());

    generator.forward_run(oracle).await.expect("forward run");
    let witness = generator
        .generate_witness(build_oracle(input).expect("build oracle"))
        .await
        .expect("generate witness");

    assert!(!witness.is_empty());
}

#[tokio::test]
async fn gpu_prover_from_fixture_block() {
    require_gpu_tests!();

    common::init_tracing();
    let input = common::load_fixture_input("24073997");
    let oracle = build_oracle(input.clone()).expect("build oracle");
    let prover = Prover::new(common::app_bin_path().as_path(), None).expect("create prover");

    let result = prover
        .prove(input.block_header.number, oracle)
        .await
        .expect("prove block");

    assert!(!result.proof_bytes.is_empty());
    assert!(result.cycles > 0);
    assert!(result.proving_time_secs > 0.0);
}
