use crate::prover::gpu_prover::ProofResult;

pub(crate) mod block_stream;
pub(crate) mod cache_manager;
pub(crate) mod cpu_witness;
pub(crate) mod eth_proofs_upload;
pub(crate) mod gpu_prove;

#[derive(Debug)]
pub(crate) enum CalculationUpdate {
    WitnessCalculated {
        block_number: u64,
        _data: Vec<u32>,
    },
    ProofQueued {
        block_number: u64,
    },
    ProofProving {
        block_number: u64,
    },
    ProofProvided {
        block_number: u64,
        proof_result: ProofResult,
    },
}
