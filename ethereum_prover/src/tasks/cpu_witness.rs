use alloy::providers::DynProvider;
use anyhow::Context as _;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use url::Url;

use crate::{
    CacheStorage,
    metrics::{InflightGuard, METRICS},
    prover::{
        cpu_witness::{CpuWitnessGenerator, DebuggerTxCallback},
        oracle::build_oracle,
        types::EthBlockInput,
    },
    tasks::CalculationUpdate,
    types::OnFailure,
};

#[derive(Debug)]
pub(crate) struct CpuWitnessTask {
    witness_generator: CpuWitnessGenerator,
    witness_receiver: Receiver<EthBlockInput>,
    command_sender: Sender<CalculationUpdate>,
    on_failure: OnFailure,
    rpc_url: Option<Url>,
    cache: CacheStorage,
}

impl CpuWitnessTask {
    pub fn new(
        witness_generator: CpuWitnessGenerator,
        witness_receiver: Receiver<EthBlockInput>,
        on_failure: OnFailure,
        rpc_url: Option<Url>,
        cache: CacheStorage,
    ) -> (Self, Receiver<CalculationUpdate>) {
        let (command_sender, command_receiver) = channel(10);
        (
            Self {
                witness_generator,
                witness_receiver,
                command_sender,
                on_failure,
                rpc_url,
                cache,
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
            let _inflight = InflightGuard::new(&METRICS.inflight_witness_tasks);
            let latency = METRICS.witness_duration.start();
            match self.process_block(witness).await {
                Ok(cpu_witness) => {
                    tracing::info!("Generated CPU witness for block {}", block_number);
                    METRICS.witness_success_total.inc();
                    latency.observe();
                    self.command_sender
                        .send(CalculationUpdate::WitnessCalculated {
                            block_number,
                            _data: cpu_witness,
                        })
                        .await?;
                }
                Err(err) => {
                    METRICS.witness_failure_total.inc();
                    latency.observe();
                    sentry::with_scope(
                        |scope| {
                            scope.set_level(Some(sentry::Level::Error));
                            scope.set_tag("mode", "cpu_witness");
                            scope.set_tag("block_number", block_number.to_string());
                        },
                        || sentry_anyhow::capture_anyhow(&err),
                    );
                    match self.on_failure {
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
                    }
                }
            }

            // We don't any commands as we're not generating proofs
        }

        Ok(())
    }

    async fn process_block(&self, witness: EthBlockInput) -> anyhow::Result<Vec<u32>> {
        tracing::info!(
            "Performing forward run for block {}",
            witness.block_header.number
        );
        let oracle = build_oracle(witness.clone())?;
        if let Err(e) = self.witness_generator.forward_run(oracle).await {
            self.debug_block(witness.clone()).await?;
            return Err(e);
        }

        tracing::info!(
            "Generating witness for block {}",
            witness.block_header.number
        );
        let oracle = build_oracle(witness)?;
        let cpu_witness = self.witness_generator.generate_witness(oracle).await?;
        Ok(cpu_witness)
    }

    async fn debug_block(&self, witness: EthBlockInput) -> anyhow::Result<()> {
        match &self.rpc_url {
            Some(rpc_url) => {
                tracing::warn!("Forward run failed, attempting to debug using RPC");
                let oracle = build_oracle(witness.clone())?;
                let provider = alloy::providers::builder().connect_http(rpc_url.clone());
                let provider = DynProvider::new(provider);

                let debugger = DebuggerTxCallback::new(
                    witness.block_header.number,
                    witness.transactions.clone(),
                    provider,
                    self.cache.clone(),
                );
                let debugger = self
                    .witness_generator
                    .debug(oracle, debugger)
                    .await
                    .with_context(|| {
                        format!("debugging failed for block {}", witness.block_header.number)
                    })?;
                tracing::info!(
                    "Debugging completed for block {}",
                    witness.block_header.number
                );
                for problem in debugger.get_problems() {
                    tracing::error!("Problem found: {}", problem);
                }
            }
            None => {
                tracing::warn!("Forward run failed, no RPC URL provided for debugging");
                tracing::warn!("In order to debug the issue, provide an RPC URL in the config");
            }
        }
        Ok(())
    }
}
