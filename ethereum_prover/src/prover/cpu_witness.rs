use std::collections::VecDeque;
use std::path::PathBuf;

use alloy::providers::DynProvider;
use alloy::providers::Provider;
use alloy::rpc::types::Transaction;
use basic_bootloader::bootloader::BasicBootloader;
use basic_bootloader::bootloader::config::BasicBootloaderForwardETHLikeConfig;
use forward_system::run::InvalidTransaction;
use forward_system::run::TxResultCallback;
use forward_system::run::result_keeper::ForwardRunningResultKeeper;
use forward_system::run::result_keeper::TxProcessingOutputOwned;
use forward_system::run::test_impl::NoopTxCallback;
use forward_system::system::system_types::ethereum::EthereumStorageSystemTypesWithPostOps;
use oracle_provider::ReadWitnessSource;
use oracle_provider::ZkEENonDeterminismSource;
use zk_ee::system::tracer::NopTracer;

use crate::CacheStorage;

#[derive(Debug, Clone)]
pub(crate) struct CpuWitnessGenerator {
    app_bin_path: PathBuf,
}

impl CpuWitnessGenerator {
    pub fn new(app_bin_path: PathBuf) -> Self {
        Self { app_bin_path }
    }

    pub async fn forward_run(&self, oracle: ZkEENonDeterminismSource) -> anyhow::Result<()> {
        tokio::task::spawn_blocking(move || {
            let mut result_keeper = ForwardRunningResultKeeper::new(NoopTxCallback);
            let mut nop_tracer = NopTracer::default();
            BasicBootloader::<EthereumStorageSystemTypesWithPostOps<ZkEENonDeterminismSource>>::run::<
                BasicBootloaderForwardETHLikeConfig,
            >(oracle, &mut result_keeper, &mut nop_tracer)
            .map_err(|err| anyhow::anyhow!("Failed to run the STF in forward run mode: {err:?}"))?;

            Ok(())
        }).await?
    }

    pub async fn debug(
        &self,
        oracle: ZkEENonDeterminismSource,
        debugger: DebuggerTxCallback,
    ) -> anyhow::Result<DebuggerTxCallback> {
        tokio::task::spawn_blocking(move || {
            let mut result_keeper = ForwardRunningResultKeeper::new(debugger);
            let mut nop_tracer = NopTracer::default();
            // We ignore the error, as we are debugging and getting the results.
            let _ = BasicBootloader::<
                EthereumStorageSystemTypesWithPostOps<ZkEENonDeterminismSource>,
            >::run::<BasicBootloaderForwardETHLikeConfig>(
                oracle,
                &mut result_keeper,
                &mut nop_tracer,
            );

            Ok(result_keeper.tx_result_callback)
        })
        .await?
    }

    pub async fn generate_witness(
        &self,
        oracle: ZkEENonDeterminismSource,
    ) -> anyhow::Result<Vec<u32>> {
        let app_bin_path = self.app_bin_path.clone();
        tokio::task::spawn_blocking(move || {
            let copy_source = ReadWitnessSource::new(oracle);
            let items = copy_source.get_read_items();

            let output = zksync_os_runner::run(app_bin_path, None, 1 << 36, copy_source);
            if output == [0u32; 8] {
                anyhow::bail!("zksync_os_runner failed to execute the block");
            }

            let witness = items.borrow().clone();
            Ok(witness)
        })
        .await?
    }
}

#[derive(Clone)]
pub(crate) struct DebuggerTxCallback {
    block_number: u64,
    txs: VecDeque<Transaction>,
    provider: DynProvider,
    problems: Vec<String>,
    cache: CacheStorage,
}

impl DebuggerTxCallback {
    pub fn new(
        block_number: u64,
        txs: Vec<Transaction>,
        provider: DynProvider,
        cache: CacheStorage,
    ) -> Self {
        Self {
            block_number,
            txs: VecDeque::from(txs),
            provider,
            problems: vec![],
            cache,
        }
    }

    pub fn get_problems(&self) -> &[String] {
        &self.problems
    }
}

impl TxResultCallback for DebuggerTxCallback {
    fn tx_executed(
        &mut self,
        tx_execution_result: Result<TxProcessingOutputOwned, InvalidTransaction>,
    ) {
        let Some(executed_tx) = self.txs.pop_front() else {
            tracing::error!("Transaction stream is empty, but tx_executed was called");
            return;
        };

        let Ok(tx_execution_result) = tx_execution_result else {
            let Err(err) = tx_execution_result else {
                return;
            };
            tracing::error!(
                "Transaction {:?} was considered invalid: {:?}",
                executed_tx.inner.tx_hash(),
                err
            );
            return;
        };

        let rt_handle = tokio::runtime::Handle::current();
        let tx_hash = executed_tx.inner.tx_hash();
        tracing::debug!("Debugging transaction {tx_hash:?}");

        let receipt = if let Ok(Some(receipt)) = self.cache.load_receipt(self.block_number, tx_hash)
        {
            receipt
        } else {
            let receipt_result =
                rt_handle.block_on(async { self.provider.get_transaction_receipt(*tx_hash).await });
            let receipt = match receipt_result {
                Ok(Some(receipt)) => receipt,
                Ok(None) => {
                    tracing::error!("Transaction receipt not found for {:?}", tx_hash);
                    return;
                }
                Err(err) => {
                    tracing::error!(
                        "Failed to get transaction receipt for {:?}: {}",
                        tx_hash,
                        err
                    );
                    return;
                }
            };

            if let Err(err) = self.cache.save_receipt(self.block_number, receipt.clone()) {
                tracing::error!("Failed to save cache entry: {err}");
            }

            receipt
        };

        tracing::debug!("Fetched receipt for transaction {tx_hash:?}");

        if tx_execution_result.status != receipt.status() {
            tracing::error!(
                "Transaction {:?} execution status mismatch: STF status = {:?}, Ethereum status = {:?}",
                tx_hash,
                tx_execution_result.status,
                receipt.status()
            );
        }
        if tx_execution_result.gas_used != receipt.gas_used {
            tracing::error!(
                "Transaction {:?} gas used mismatch: STF gas used = {}, Ethereum gas used = {}",
                tx_hash,
                tx_execution_result.gas_used,
                receipt.gas_used
            );
        }
    }
}
