use anyhow::Context as _;
use oracle_provider::ZkEENonDeterminismSource;
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::Instant;

#[derive(Debug)]
pub struct ProofResult {
    pub proof_bytes: Vec<u8>,
    pub cycles: u64,
    pub proving_time_secs: f64,
}

pub struct Prover {
    app_bin_path: PathBuf,
    worker_threads: Option<usize>,
    inner: Arc<Mutex<execution_utils::unrolled_gpu::UnrolledProver>>,
}

impl std::fmt::Debug for Prover {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("Prover").finish()
    }
}

impl Prover {
    pub fn new(app_bin_path: &Path, worker_threads: Option<usize>) -> anyhow::Result<Self> {
        let inner = create_unrolled_prover(app_bin_path, worker_threads).with_context(|| {
            format!(
                "failed to create unrolled prover with app binary at {:?}",
                app_bin_path
            )
        })?;
        Ok(Self {
            app_bin_path: app_bin_path.to_path_buf(),
            worker_threads,
            inner: Arc::new(Mutex::new(inner)),
        })
    }

    pub async fn prove(
        &self,
        block_number: u64,
        oracle: ZkEENonDeterminismSource,
    ) -> anyhow::Result<ProofResult> {
        let start = Instant::now();

        let inner = self.inner.clone();

        let future_result = tokio::task::spawn_blocking(move || {
            let prover = inner.lock().unwrap();
            prover.prove(block_number, oracle)
        })
        .await;
        let (proof, cycles) = match future_result {
            Ok(result) => result,
            Err(err) => {
                let panic_msg = crate::utils::extract_panic_message(err);

                // If prover panics, it is not safe to use it again, since some of threads may be poisoned/dead.
                // We need to re-instantiate it.
                {
                    let mut inner = self.inner.lock().unwrap();
                    // If we cannot reinstantiate the prover for some reason, we cannot do much -- better to panic.
                    *inner = create_unrolled_prover(self.app_bin_path.as_path(), self.worker_threads).expect(
                        "failed to re-instantiate prover after panic; prover app binary path is invalid",
                    );
                }

                return Err(anyhow::anyhow!(
                    "Prover task panicked for the block {}: {}",
                    block_number,
                    panic_msg
                ));
            }
        };

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

fn create_unrolled_prover(
    app_bin_path: &Path,
    worker_threads: Option<usize>,
) -> anyhow::Result<execution_utils::unrolled_gpu::UnrolledProver> {
    let base_path = strip_bin_suffix(app_bin_path)?;
    let mut configuration =
        execution_utils::gpu_prover::execution::prover::ExecutionProverConfiguration::default();
    if let Some(threads) = worker_threads {
        configuration.max_thread_pool_threads = Some(threads);
        configuration.replay_worker_threads_count = threads;
    }

    let unrolled_prover = execution_utils::unrolled_gpu::UnrolledProver::new(
        &base_path,
        configuration,
        execution_utils::unrolled_gpu::UnrolledProverLevel::RecursionUnified,
    );
    Ok(unrolled_prover)
}
