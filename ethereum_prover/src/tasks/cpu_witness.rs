use alloy::providers::DynProvider;
use anyhow::Context as _;
use tokio::sync::mpsc::{Receiver, Sender, channel};
use url::Url;

use crate::{
    CacheStorage,
    metrics::{InflightGuard, METRICS},
    observability,
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
            let block_number = witness.block_header.number;
            observability::bind_block("cpu_witness", block_number, async {
                let result = async {
                    tracing::info!("Generating CPU witness for block {}", block_number);
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
                                .await
                                .with_context(|| {
                                    format!(
                                        "failed to forward witness result for block {block_number}"
                                    )
                                })?;
                        }
                        Err(err) => {
                            METRICS.witness_failure_total.inc();
                            latency.observe();
                            match self.on_failure {
                                OnFailure::Exit => {
                                    return Err(err).with_context(|| {
                                        format!(
                                            "Failed to generate witness for the block {block_number}"
                                        )
                                    });
                                }
                                OnFailure::Continue => {
                                    observability::capture_anyhow(&err);
                                    tracing::error!(
                                        "Failed to generate witness for the block {block_number}: {err}"
                                    );
                                }
                            }
                        }
                    }

                    // We don't emit commands in CPU witness mode beyond the witness payload itself.
                    Ok(())
                }
                .await;

                if let Err(ref err) = result {
                    observability::capture_anyhow(err);
                }

                result
            })
            .await?;
        }

        Ok(())
    }

    async fn process_block(&self, witness: EthBlockInput) -> anyhow::Result<Vec<u32>> {
        let block_number = witness.block_header.number;
        tracing::info!(
            "Performing forward run for block {}",
            block_number
        );
        let oracle = build_oracle(witness.clone())
            .with_context(|| format!("failed to build the forward-run oracle for block {block_number}"))?;
        if let Err(err) = self
            .witness_generator
            .forward_run(block_number, oracle)
            .await
            .with_context(|| format!("failed to perform forward run for block {block_number}"))
        {
            self.debug_block(witness.clone())
                .await
                .with_context(|| format!("failed to debug block {block_number} after forward-run failure"))?;
            return Err(err);
        }

        tracing::info!("Generating witness for block {}", block_number);
        let oracle = build_oracle(witness)
            .with_context(|| format!("failed to build the witness oracle for block {block_number}"))?;
        let cpu_witness = self
            .witness_generator
            .generate_witness(block_number, oracle)
            .await
            .with_context(|| format!("failed to generate witness data for block {block_number}"))?;
        Ok(cpu_witness)
    }

    async fn debug_block(&self, witness: EthBlockInput) -> anyhow::Result<()> {
        let block_number = witness.block_header.number;
        match &self.rpc_url {
            Some(rpc_url) => {
                tracing::warn!("Forward run failed for block {block_number}, attempting to debug using RPC");
                let oracle = build_oracle(witness.clone())
                    .with_context(|| format!("failed to build the debug oracle for block {block_number}"))?;
                let provider = alloy::providers::builder().connect_http(rpc_url.clone());
                let provider = DynProvider::new(provider);

                let debugger = DebuggerTxCallback::new(
                    block_number,
                    witness.transactions.clone(),
                    provider,
                    self.cache.clone(),
                );
                let debugger = self
                    .witness_generator
                    .debug(block_number, oracle, debugger)
                    .await
                    .with_context(|| {
                        format!("debugging failed for block {block_number}")
                    })?;
                tracing::info!("Debugging completed for block {}", block_number);
                for problem in debugger.get_problems() {
                    tracing::error!("Problem found while debugging block {block_number}: {problem}");
                }
            }
            None => {
                tracing::warn!("Forward run failed for block {block_number}, no RPC URL provided for debugging");
                tracing::warn!("In order to debug the issue, provide an RPC URL in the config");
            }
        }
        Ok(())
    }
}
