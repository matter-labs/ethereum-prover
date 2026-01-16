use crate::cache::CacheStorage;
use crate::tasks::CalculationUpdate;
use crate::types::CachePolicy;
use tokio::sync::mpsc::{Receiver, Sender, channel};

#[derive(Debug)]
pub(crate) struct CacheManagerTask {
    command_mode_receiver: Receiver<CalculationUpdate>,
    command_mode_sender: Sender<CalculationUpdate>,
    cache_storage: CacheStorage,
    cache_policy: CachePolicy,
}

impl CacheManagerTask {
    pub fn new(
        receiver: Receiver<CalculationUpdate>,
        cache_storage: CacheStorage,
        cache_policy: CachePolicy,
    ) -> (Self, Receiver<CalculationUpdate>) {
        let (command_mode_sender, command_mode_receiver) = channel(10);
        (
            Self {
                command_mode_receiver: receiver,
                command_mode_sender,
                cache_storage,
                cache_policy,
            },
            command_mode_receiver,
        )
    }

    pub async fn run(mut self) -> anyhow::Result<()> {
        while let Some(command) = self.command_mode_receiver.recv().await {
            match &command {
                CalculationUpdate::ProofProvided { block_number, .. }
                | CalculationUpdate::WitnessCalculated { block_number, .. } => {
                    if matches!(self.cache_policy, CachePolicy::OnFailure) {
                        self.cache_storage.remove_cached_block(*block_number)?;
                    }
                }
                _ => {}
            }
            self.command_mode_sender.send(command).await?;
        }

        Ok(())
    }
}
