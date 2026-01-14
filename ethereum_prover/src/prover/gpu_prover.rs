use oracle_provider::ZkEENonDeterminismSource;
use std::path::Path;
use std::time::Instant;

#[derive(Debug)]
pub struct ProofResult {
    pub proof_bytes: Vec<u8>,
    pub cycles: u64,
    pub proving_time_secs: f64,
}

pub struct Prover {
    inner: execution_utils::unrolled_gpu::UnrolledProver,
}

impl std::fmt::Debug for Prover {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prover").finish()
    }
}

impl Prover {
    pub fn new(app_bin_path: &Path, worker_threads: Option<usize>) -> anyhow::Result<Self> {
        let base_path = strip_bin_suffix(app_bin_path)?;
        let mut configuration =
            execution_utils::gpu_prover::execution::prover::ExecutionProverConfiguration::default();
        if let Some(threads) = worker_threads {
            configuration.max_thread_pool_threads = Some(threads);
            configuration.replay_worker_threads_count = threads;
        }
        let inner = execution_utils::unrolled_gpu::UnrolledProver::new(
            &base_path,
            configuration,
            execution_utils::unrolled_gpu::UnrolledProverLevel::RecursionUnified,
        );
        Ok(Self { inner })
    }

    pub fn prove(
        &self,
        block_number: u64,
        oracle: ZkEENonDeterminismSource,
    ) -> anyhow::Result<ProofResult> {
        let start = Instant::now();
        let (proof, cycles) = self.inner.prove(block_number, oracle);
        let proving_time_secs = start.elapsed().as_secs_f64();
        let proof_bytes = bincode::serde::encode_to_vec(&proof, bincode::config::standard())?;
        Ok(ProofResult {
            proof_bytes,
            cycles,
            proving_time_secs,
        })
    }
}

fn strip_bin_suffix(path: &Path) -> anyhow::Result<String> {
    let path_str = path
        .to_str()
        .ok_or_else(|| anyhow::anyhow!("app path is not valid UTF-8"))?;
    if let Some(stripped) = path_str.strip_suffix(".bin") {
        Ok(stripped.to_string())
    } else {
        Ok(path_str.to_string())
    }
}
