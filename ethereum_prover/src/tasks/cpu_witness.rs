use anyhow::Context as _;
use tokio::sync::mpsc::{Receiver, Sender, channel};

use crate::{
    prover::{cpu_witness::CpuWitnessGenerator, oracle::build_oracle, types::EthBlockInput},
    tasks::CalculationUpdate,
    types::OnFailure,
};

#[derive(Debug)]
pub(crate) struct CpuWitnessTask {
    witness_generator: CpuWitnessGenerator,
    witness_receiver: Receiver<EthBlockInput>,
    command_sender: Sender<CalculationUpdate>,
    on_failure: OnFailure,
}

impl CpuWitnessTask {
    pub fn new(
        witness_generator: CpuWitnessGenerator,
        witness_receiver: Receiver<EthBlockInput>,
        on_failure: OnFailure,
    ) -> (Self, Receiver<CalculationUpdate>) {
        let (command_sender, command_receiver) = channel(10);
        (
            Self {
                witness_generator,
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
                "Generating CPU witness for block {}",
                witness.block_header.number
            );
            let block_number = witness.block_header.number;
            match self.process_block(witness).await {
                Ok(cpu_witness) => {
                    tracing::info!("Generated CPU witness for block {}", block_number);
                    self.command_sender
                        .send(CalculationUpdate::WitnessCalculated {
                            block_number,
                            _data: cpu_witness,
                        })
                        .await?;
                }
                Err(err) => match self.on_failure {
                    OnFailure::Exit => {
                        return Err(err).with_context(|| {
                            format!("Failed to generate witness for the block {block_number}")
                        });
                    }
                    OnFailure::Continue => {
                        tracing::error!(
                            "Failed to generate witness for the block {block_number}: {err}"
                        );
                        continue;
                    }
                },
            }

            // We don't any commands as we're not generating proofs
        }

        Ok(())
    }

    async fn process_block(&self, witness: EthBlockInput) -> anyhow::Result<Vec<u32>> {
        let oracle = build_oracle(witness)?;
        // TODO: not the best idea
        let witgen = self.witness_generator.clone();

        let cpu_witness =
            tokio::task::spawn_blocking(move || witgen.generate_witness(oracle)).await??;
        Ok(cpu_witness)
    }
}
