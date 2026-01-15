use alloy::providers::{DynProvider, Provider};
use tokio::sync::mpsc::{Receiver, Sender, channel};
use url::Url;

use crate::{CacheStorage, prover::types::EthBlockInput, types::CachePolicy};

#[derive(Debug)]
pub struct SingleBlockStream {
    block_number: Option<u64>,
    rpc_url: Option<Url>,
    cache: CacheStorage,
    cache_policy: CachePolicy,
    sender: Sender<EthBlockInput>,
}

impl SingleBlockStream {
    pub fn new(
        block_number: Option<u64>,
        rpc_url: Option<Url>,
        cache: CacheStorage,
        cache_policy: CachePolicy,
    ) -> (Self, Receiver<EthBlockInput>) {
        let (sender, receiver) = channel(1);
        (
            Self {
                block_number,
                rpc_url,
                cache,
                cache_policy,
                sender,
            },
            receiver,
        )
    }

    pub async fn run(self) -> anyhow::Result<()> {
        tracing::info!("Running single block stream");

        let input = if let Some(block_number) = self.block_number
            && self.cache.has_cached_block(block_number)
        {
            tracing::info!("Loading block {block_number} from cache");
            let Some((block, witness)) = self.cache.load_block(block_number)? else {
                anyhow::bail!("cache indicated block {block_number} exists, but contents missing");
            };
            EthBlockInput::new(block, witness)
        } else {
            tracing::info!("Block number is unknown or not cached, fetching from RPC");
            let Some(rpc_url) = self.rpc_url else {
                anyhow::bail!("Block number not cached and no RPC URL provided");
            };

            let provider = alloy::providers::ProviderBuilder::new().connect_http(rpc_url);
            let provider = DynProvider::new(provider);

            let block_number = match self.block_number {
                Some(block_number) => block_number,
                None => provider.get_block_number().await?,
            };

            tracing::info!("Fetching block {}", block_number);

            super::fetch_input(&provider, block_number, self.cache_policy, &self.cache).await?
        };

        tracing::info!(
            "Sending block input for block {}",
            input.block_header.number
        );
        self.sender.send(input).await?;
        // We're sending single block only, so we can close the sender here.
        Ok(())
    }
}
