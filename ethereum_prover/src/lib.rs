#![feature(allocator_api)]

use smart_config::value::ExposeSecret;

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
pub(crate) mod prover;
pub(crate) mod tasks;
pub(crate) mod types;

#[derive(Debug, Default)]
pub struct Runner {}

impl Runner {
    pub fn new() -> Self {
        Self::default()
    }

    pub async fn run(self, cli: Cli, config: EthProverConfig) -> anyhow::Result<()> {
        let mut tasks = Vec::new();

        let cache_storage = CacheStorage::new(".cache");

        let (block_stream_task, block_stream_receiver) = match cli.command {
            Command::Run => {
                let Some(rpc_url) = config.rpc_url.clone() else {
                    anyhow::bail!("RPC URL is required for continuous mode");
                };

                // Create and run continuous block stream
                let (stream, receiver) = tasks::block_stream::ContinuousBlockStream::new(
                    rpc_url.expose_secret().to_string(),
                    config.prover_id,
                    config.block_mod,
                    cache_storage.clone(),
                    config.cache_policy,
                );
                (tokio::spawn(stream.run()), receiver)
            }
            Command::Block { block_number } => {
                let (stream, receiver) = tasks::block_stream::SingleBlockStream::new(
                    block_number,
                    config
                        .rpc_url
                        .clone()
                        .map(|u| u.expose_secret().to_string()),
                    cache_storage.clone(),
                    config.cache_policy,
                );
                (tokio::spawn(stream.run()), receiver)
            }
        };
        tasks.push(block_stream_task);

        let (mode_task, mode_command_receiver) = match config.mode {
            Mode::CpuWitness => {
                let cpu_witness_generator = CpuWitnessGenerator::new(config.app_bin_path);
                let (task, command_receiver) = tasks::cpu_witness::CpuWitnessTask::new(
                    cpu_witness_generator,
                    block_stream_receiver,
                    config.on_failure,
                );
                (tokio::spawn(task.run()), command_receiver)
            }
            Mode::GpuProve => {
                // TODO 1: support worker threads
                // TODO 2: this is blocking, use `tokio::task::spawn_blocking`?
                let gpu_prover =
                    Prover::new(config.app_bin_path.as_path(), None).expect("Cannot create prover");
                let (task, command_receiver) = tasks::gpu_prove::GpuProveTask::new(
                    gpu_prover,
                    block_stream_receiver,
                    config.on_failure,
                );
                (tokio::spawn(task.run()), command_receiver)
            }
        };
        tasks.push(mode_task);

        let (cache_manager_task, mode_command_receiver) = {
            let (task, mode_command_receiver) = tasks::cache_manager::CacheManagerTask::new(
                mode_command_receiver,
                cache_storage,
                config.cache_policy,
            );
            (tokio::spawn(task.run()), mode_command_receiver)
        };
        tasks.push(cache_manager_task);

        let submission_task = if config.ethproofs_submission.enabled() {
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
            tokio::spawn(task.run())
        } else {
            let task = tasks::eth_proofs_upload::EthProofsNoOpTask::new(mode_command_receiver);
            tokio::spawn(task.run())
        };
        tasks.push(submission_task);

        let results = futures::future::join_all(tasks).await; // Wait until the stream is exhausted

        for result in results {
            result??; // TODO: better handling
        }

        Ok(())
    }
}
