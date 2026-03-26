use crate::{
    metrics::{InflightGuard, METRICS},
    observability,
    prover::{gpu_prover::ProofResult, oracle::build_oracle, types::EthBlockInput},
    tasks::CalculationUpdate,
    types::OnFailure,
};
use anyhow::Context as _;
use tokio::sync::mpsc::{Receiver, Sender, channel};

use crate::prover::gpu_prover::Prover;

#[derive(Debug)]
pub(crate) struct GpuProveTask {
    gpu_prover: Prover,
    witness_receiver: Receiver<EthBlockInput>,
    command_sender: Sender<CalculationUpdate>,
    on_failure: OnFailure,
}

impl GpuProveTask {
    pub fn new(
        gpu_prover: Prover,
        witness_receiver: Receiver<EthBlockInput>,
        on_failure: OnFailure,
    ) -> (Self, Receiver<CalculationUpdate>) {
        let (command_sender, command_receiver) = channel(10);
        (
            Self {
                gpu_prover,
                witness_receiver,
                command_sender,
                on_failure,
            },
            command_receiver,
        )
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        while let Some(witness) = self.witness_receiver.recv().await {
            let block_number = witness.block_header.number;
            observability::bind_block("gpu_prove", block_number, async {
                let result = async {
                    tracing::info!("Generating GPU proof for block {}", block_number);
                    let _inflight = InflightGuard::new(&METRICS.inflight_proof_tasks);
                    let latency = METRICS.proof_duration.start();
                    self.command_sender
                        .send(CalculationUpdate::ProofQueued { block_number })
                        .await
                        .with_context(|| {
                            format!("failed to mark block {block_number} as queued in the pipeline")
                        })?;
                    self.command_sender
                        .send(CalculationUpdate::ProofProving { block_number })
                        .await
                        .with_context(|| {
                            format!(
                                "failed to mark block {block_number} as proving in the pipeline"
                            )
                        })?;

                    match self.process_block(witness).await {
                        Ok(proof_result) => {
                            tracing::info!(
                                "Generated GPU proof for block {}. Number of cycles: {}, proving time: {}s",
                                block_number,
                                proof_result.cycles,
                                proof_result.proving_time_secs
                            );
                            METRICS.proof_success_total.inc();
                            latency.observe();
                            self.command_sender
                                .send(CalculationUpdate::ProofProvided {
                                    block_number,
                                    proof_result,
                                })
                                .await
                                .with_context(|| {
                                    format!("failed to forward proof result for block {block_number}")
                                })?;
                        }
                        Err(err) => {
                            METRICS.proof_failure_total.inc();
                            latency.observe();
                            match self.on_failure {
                                OnFailure::Exit => {
                                    return Err(err).with_context(|| {
                                        format!(
                                            "Failed to generate proof for the block {block_number}"
                                        )
                                    });
                                }
                                OnFailure::Continue => {
                                    observability::capture_anyhow(&err);
                                    tracing::error!(
                                        "Failed to generate proof for the block {block_number}: {err}"
                                    );
                                }
                            }
                        }
                    }

                    Ok(())
                }
                .await;

                if let Err(ref err) = result {
                    observability::capture_anyhow(err);
                }

                result
            })
            .await?;
        }

        Ok(())
    }

    async fn process_block(&mut self, witness: EthBlockInput) -> anyhow::Result<ProofResult> {
        let block_number = witness.block_header.number;
        let oracle = build_oracle(witness)
            .with_context(|| format!("failed to build the proving oracle for block {block_number}"))?;

        tracing::info!("Proving block {} on GPU", block_number);
        self.gpu_prover
            .prove(block_number, oracle)
            .await
            .with_context(|| format!("failed to prove block {block_number}"))
    }
}
