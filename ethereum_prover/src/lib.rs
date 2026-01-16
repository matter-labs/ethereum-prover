#![feature(allocator_api)]

use anyhow::Context as _;
use smart_config::value::ExposeSecret;
use url::Url;

use crate::{
    cache::CacheStorage,
    clients::ethproofs::EthproofsClient,
    config::{Cli, Command, EthProverConfig},
    prover::{cpu_witness::CpuWitnessGenerator, gpu_prover::Prover},
    types::Mode,
};

pub mod config;

pub(crate) mod cache;
pub(crate) mod clients;
pub mod metrics;
pub mod prover;
pub(crate) mod tasks;
pub(crate) mod types;
pub(crate) mod utils;

#[derive(Debug, Default)]
pub struct Runner {}

impl Runner {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn run(self, cli: Cli, config: EthProverConfig) -> anyhow::Result<()> {
        let mut join_set = tokio::task::JoinSet::new();

        let cache_storage = CacheStorage::new(".cache").context("failed to initialize cache")?;
        let rpc_url = config
            .rpc_url
            .clone()
            .map(|u| u.expose_secret().to_string())
            .map(|u| u.parse::<Url>().context("invalid RPC URL"))
            .transpose()?;

        let (block_stream_receiver, should_create_cache_manager) = match cli.command {
            Command::Run => {
                let Some(rpc_url) = rpc_url.clone() else {
                    anyhow::bail!("RPC URL is required for continuous mode");
                };

                // Create and run continuous block stream
                let (stream, receiver) = tasks::block_stream::ContinuousBlockStream::new(
                    rpc_url,
                    config.prover_id,
                    config.block_mod,
                    cache_storage.clone(),
                    config.cache_policy,
                );
                join_set.spawn(stream.run());
                (receiver, true)
            }
            Command::Block { block_number } => {
                let (stream, receiver) = tasks::block_stream::SingleBlockStream::new(
                    block_number,
                    rpc_url.clone(),
                    cache_storage.clone(),
                    config.cache_policy,
                );
                // Single block mode is used for debugging, so we don't want to remove cache artifacts
                join_set.spawn(stream.run());
                (receiver, false)
            }
        };

        let mut mode_command_receiver = match config.mode {
            Mode::CpuWitness => {
                let cpu_witness_generator = CpuWitnessGenerator::new(config.app_bin_path);
                let (task, command_receiver) = tasks::cpu_witness::CpuWitnessTask::new(
                    cpu_witness_generator,
                    block_stream_receiver,
                    config.on_failure,
                    rpc_url.clone(),
                    cache_storage.clone(),
                );
                join_set.spawn(task.run());
                command_receiver
            }
            Mode::GpuProve => {
                // TODO: support worker threads? Though it's likely not needed anytime soon.
                tracing::info!("Creating GPU prover");
                let app_bin_path = config.app_bin_path.clone();
                let gpu_prover = tokio::task::spawn_blocking(move || {
                    Prover::new(app_bin_path.as_path(), None).context("failed to create prover")
                })
                .await
                .context("prover creation task panicked")??;
                tracing::info!("GPU prover created");

                let (task, command_receiver) = tasks::gpu_prove::GpuProveTask::new(
                    gpu_prover,
                    block_stream_receiver,
                    config.on_failure,
                );
                join_set.spawn(task.run());
                command_receiver
            }
        };

        if should_create_cache_manager {
            let (cache_manager_task, new_command_receiver) = {
                let (task, mode_command_receiver) = tasks::cache_manager::CacheManagerTask::new(
                    mode_command_receiver,
                    cache_storage,
                    config.cache_policy,
                );
                (task, mode_command_receiver)
            };
            mode_command_receiver = new_command_receiver;
            join_set.spawn(cache_manager_task.run());
        }

        if config.ethproofs_submission.enabled() {
            let Some(token) = config.ethproofs_token.clone() else {
                anyhow::bail!("EthProofs submission token is required when submission is enabled");
            };

            let Some(cluster_id) = config.ethproofs_cluster_id else {
                anyhow::bail!("EthProofs cluster ID is required when submission is enabled");
            };

            let ethproofs_client = EthproofsClient::new(
                config.ethproofs_submission.is_staging(),
                token.expose_secret().to_string(),
                cluster_id,
            );
            let task = tasks::eth_proofs_upload::EthProofsUploadTask::new(
                ethproofs_client,
                mode_command_receiver,
            );
            join_set.spawn(task.run());
        } else {
            let task = tasks::eth_proofs_upload::EthProofsNoOpTask::new(mode_command_receiver);
            join_set.spawn(task.run());
        }

        while let Some(result) = join_set.join_next().await {
            match result {
                Ok(Ok(())) => {}
                Ok(Err(err)) => {
                    join_set.abort_all();
                    return Err(err);
                }
                Err(err) => {
                    join_set.abort_all();
                    return Err(err.into());
                }
            }
        }

        Ok(())
    }
}
