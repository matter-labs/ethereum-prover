use crate::clients::ethproofs::EthproofsClient;
use crate::tasks::CalculationUpdate;
use tokio::sync::mpsc::Receiver;

#[derive(Debug)]
pub(crate) struct EthProofsNoOpTask {
    command_mode_receiver: Receiver<CalculationUpdate>,
}

impl EthProofsNoOpTask {
    pub fn new(command_mode_receiver: Receiver<CalculationUpdate>) -> Self {
        Self {
            command_mode_receiver,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        while let Some(_command) = self.command_mode_receiver.recv().await {
            // No-op
        }

        Ok(())
    }
}

#[derive(Debug)]
pub(crate) struct EthProofsUploadTask {
    client: EthproofsClient,
    command_mode_receiver: Receiver<CalculationUpdate>,
}

impl EthProofsUploadTask {
    pub fn new(
        client: EthproofsClient,
        command_mode_receiver: Receiver<CalculationUpdate>,
    ) -> Self {
        Self {
            client,
            command_mode_receiver,
        }
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        while let Some(command) = self.command_mode_receiver.recv().await {
            match command {
                CalculationUpdate::ProofQueued { block_number } => {
                    tracing::info!("Marking block {block_number} as queued");
                    if let Err(err) = self.client.queue_proof(block_number).await {
                        tracing::error!("Failed to mark block {block_number} as queued: {err}");
                    }
                    tracing::info!("Block {block_number} marked as queued");
                }
                CalculationUpdate::ProofProving { block_number } => {
                    tracing::info!("Marking block {block_number} as proving");
                    if let Err(err) = self.client.proving_proof(block_number).await {
                        tracing::error!("Failed to mark block {block_number} as proving: {err}");
                    }
                    tracing::info!("Block {block_number} marked as proving");
                }
                CalculationUpdate::ProofProvided {
                    block_number,
                    proof_result,
                } => {
                    tracing::info!("Uploading proof for block {block_number}");
                    if let Err(err) = self
                        .client
                        .send_proof(
                            block_number,
                            &proof_result.proof_bytes,
                            proof_result.proving_time_secs,
                            proof_result.cycles,
                        )
                        .await
                    {
                        tracing::error!("Failed to upload proof for block {block_number}: {err}");
                    }
                    tracing::info!("Uploaded proof for block {block_number}");
                }
                _ => {
                    // Ignore other commands
                }
            }
        }

        Ok(())
    }
}
