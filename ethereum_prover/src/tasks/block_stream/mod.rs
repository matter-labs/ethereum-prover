use crate::{CacheStorage, prover::types::EthBlockInput, types::CachePolicy};
use alloy::{
    eips::BlockNumberOrTag,
    providers::{DynProvider, Provider, ext::DebugApi as _},
};

mod continuous;
mod single_block;

pub(crate) use continuous::ContinuousBlockStream;
pub(crate) use single_block::SingleBlockStream;

async fn fetch_input(
    provider: &DynProvider,
    block_number: u64,
    cache_policy: CachePolicy,
    cache: &CacheStorage,
) -> anyhow::Result<EthBlockInput> {
    let block = provider
        .get_block_by_number(BlockNumberOrTag::Number(block_number))
        .full()
        .await?
        .ok_or_else(|| anyhow::anyhow!("block {block_number} not found"))?;
    let witness = provider
        .debug_execution_witness(BlockNumberOrTag::Number(block_number))
        .await?;
    if !matches!(cache_policy, CachePolicy::Off) {
        cache.cache_block(block_number, &block, &witness)?;
    }
    let input = EthBlockInput::new(block.clone(), witness.clone());
    Ok(input)
}
