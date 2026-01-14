use crate::{
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
            tracing::info!(
                "Generating GPU proof for block {}",
                witness.block_header.number
            );
            let block_number = witness.block_header.number;
            self.command_sender
                .send(CalculationUpdate::ProofQueued { block_number })
                .await
                .ok();
            self.command_sender
                .send(CalculationUpdate::ProofProving { block_number })
                .await
                .ok();

            match self.process_block(witness).await {
                Ok(proof_result) => {
                    tracing::info!("Generated GPU proof for block {}", block_number);
                    self.command_sender
                        .send(CalculationUpdate::ProofProvided {
                            block_number,
                            proof_result,
                        })
                        .await
                        .ok();
                }
                Err(err) => match self.on_failure {
                    OnFailure::Exit => {
                        return Err(err).with_context(|| {
                            format!("Failed to generate proof for the block {block_number}")
                        });
                    }
                    OnFailure::Continue => {
                        tracing::error!(
                            "Failed to generate proof for the block {block_number}: {err}"
                        );
                        continue;
                    }
                },
            }
        }

        Ok(())
    }

    async fn process_block(&self, witness: EthBlockInput) -> anyhow::Result<ProofResult> {
        let block_number = witness.block_header.number;
        let oracle = build_oracle(witness)?;

        tracing::info!("Proving block {} on GPU", block_number);
        self.gpu_prover.prove(block_number, oracle).await
    }
}
