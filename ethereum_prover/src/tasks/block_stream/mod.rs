use std::{future::Future, time::Duration};

use anyhow::Context as _;

use crate::{CacheStorage, prover::types::EthBlockInput, types::CachePolicy};
use alloy::{
    eips::BlockNumberOrTag,
    providers::{DynProvider, Provider, ext::DebugApi as _},
};

mod continuous;
mod single_block;

pub(crate) use continuous::ContinuousBlockStream;
pub(crate) use single_block::SingleBlockStream;

const MAX_RPC_ATTEMPTS: usize = 3;
const BASE_RPC_BACKOFF_MS: u64 = 200;

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

async fn fetch_input_with_retries(
    provider: &DynProvider,
    block_number: u64,
    cache_policy: CachePolicy,
    cache: &CacheStorage,
) -> anyhow::Result<EthBlockInput> {
    let operation = format!("fetch block input for block {block_number}");
    retry_rpc_call(&operation, || async {
        fetch_input(provider, block_number, cache_policy, cache).await
    })
    .await
}

async fn retry_rpc_call<T, F, Fut>(operation: &str, call: F) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    retry_rpc_call_with_config(operation, MAX_RPC_ATTEMPTS, BASE_RPC_BACKOFF_MS, call).await
}

async fn retry_rpc_call_with_config<T, F, Fut>(
    operation: &str,
    max_attempts: usize,
    base_backoff_ms: u64,
    mut call: F,
) -> anyhow::Result<T>
where
    F: FnMut() -> Fut,
    Fut: Future<Output = anyhow::Result<T>>,
{
    anyhow::ensure!(
        max_attempts > 0,
        "retry policy requires at least one attempt"
    );

    for attempt in 1..=max_attempts {
        match call().await {
            Ok(value) => return Ok(value),
            Err(err) if attempt < max_attempts => {
                let backoff_ms = base_backoff_ms.saturating_mul(1_u64 << (attempt - 1));
                tracing::warn!(
                    "{operation} failed: {err}. Retrying attempt {}/{} in {}ms",
                    attempt + 1,
                    max_attempts,
                    backoff_ms
                );
                tokio::time::sleep(Duration::from_millis(backoff_ms)).await;
            }
            Err(err) => return Err(err).with_context(|| operation.to_owned()),
        }
    }

    unreachable!("retry loop always returns on success or on the final failure")
}

#[cfg(test)]
mod tests {
    use std::future;

    use super::retry_rpc_call_with_config;

    #[tokio::test]
    async fn retry_rpc_call_retries_until_success() {
        let mut attempts = 0;

        let value = retry_rpc_call_with_config("fetch block", 3, 0, || {
            attempts += 1;
            future::ready(if attempts < 3 {
                Err(anyhow::anyhow!("transient RPC error"))
            } else {
                Ok(42_u64)
            })
        })
        .await
        .expect("retry succeeds");

        assert_eq!(value, 42);
        assert_eq!(attempts, 3);
    }

    #[tokio::test]
    async fn retry_rpc_call_returns_final_error() {
        let mut attempts = 0;

        let err = retry_rpc_call_with_config("fetch head", 3, 0, || {
            attempts += 1;
            future::ready(Err::<u64, _>(anyhow::anyhow!("still failing")))
        })
        .await
        .expect_err("retry should fail");

        assert_eq!(attempts, 3);
        assert!(err.to_string().contains("fetch head"));
        assert!(
            err.chain()
                .any(|cause| cause.to_string().contains("still failing"))
        );
    }
}
