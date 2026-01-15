use std::time::Duration;

use alloy::providers::{DynProvider, Provider};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use url::Url;

use crate::metrics::METRICS;
use crate::{CacheStorage, prover::types::EthBlockInput, types::CachePolicy};

const POLL_INTERVAL_SECS: u64 = 2;

#[derive(Debug)]
pub struct ContinuousBlockStream {
    prover_id: u64,
    block_mod: u64,
    provider: DynProvider,
    cache: CacheStorage,
    cache_policy: CachePolicy,
    sender: Sender<EthBlockInput>,
}

impl ContinuousBlockStream {
    pub fn new(
        rpc_url: Url,
        prover_id: u64,
        block_mod: u64,
        cache: CacheStorage,
        cache_policy: CachePolicy,
    ) -> (Self, Receiver<EthBlockInput>) {
        let (sender, receiver) = channel(10);

        let provider = alloy::providers::ProviderBuilder::new().connect_http(rpc_url);
        let provider = DynProvider::new(provider);

        (
            Self {
                provider,
                sender,
                prover_id,
                block_mod,
                cache,
                cache_policy,
            },
            receiver,
        )
    }

    pub async fn run(self) -> anyhow::Result<()> {
        tracing::info!("Running continuous block stream");
        let mut last_selected = None;
        loop {
            let head = self.provider.get_block_number().await?;
            let selected = select_block(head, self.prover_id, self.block_mod)?;
            if last_selected.is_some_and(|prev| selected <= prev) {
                tokio::time::sleep(Duration::from_secs(POLL_INTERVAL_SECS)).await;
                continue;
            }
            last_selected = Some(selected);

            tracing::info!("Selected block {}", selected);

            let eth_block_input =
                super::fetch_input(&self.provider, selected, self.cache_policy, &self.cache)
                    .await?;
            tracing::info!("Fetched block input for block {}", selected);
            METRICS.blocks_received_total.inc();
            METRICS.last_processed_block.set(selected);
            self.sender.send(eth_block_input).await?;
        }
    }
}

fn select_block(candidate_block: u64, prover_id: u64, block_mod: u64) -> anyhow::Result<u64> {
    anyhow::ensure!(block_mod > 0, "block_mod must be greater than 0");
    anyhow::ensure!(
        prover_id < block_mod,
        "prover_id must be less than block_mod"
    );
    anyhow::ensure!(
        candidate_block >= prover_id,
        "candidate block is below prover_id selection window"
    );
    let selected = candidate_block - (candidate_block % block_mod) + prover_id;
    if selected > candidate_block {
        Ok(selected - block_mod)
    } else {
        Ok(selected)
    }
}

#[cfg(test)]
mod tests {
    use super::select_block;

    #[test]
    fn select_block_matches_expected() {
        assert_eq!(select_block(100, 0, 10).unwrap(), 100);
        assert_eq!(select_block(100, 5, 10).unwrap(), 95);
        assert_eq!(select_block(100, 9, 10).unwrap(), 99);
        assert_eq!(select_block(105, 0, 10).unwrap(), 100);
        assert_eq!(select_block(105, 5, 10).unwrap(), 105);
    }

    #[test]
    fn select_block_rejects_zero_mod() {
        let err = select_block(10, 0, 0).unwrap_err();
        assert!(err.to_string().contains("block_mod"));
    }

    #[test]
    fn select_block_rejects_large_prover_id() {
        let err = select_block(10, 10, 10).unwrap_err();
        assert!(err.to_string().contains("prover_id"));
    }

    #[test]
    fn select_block_rejects_candidate_before_prover_id() {
        let err = select_block(3, 5, 10).unwrap_err();
        assert!(err.to_string().contains("candidate block"));
    }
}
