use std::path::PathBuf;

use anyhow::bail;
use oracle_provider::ReadWitnessSource;
use oracle_provider::ZkEENonDeterminismSource;

#[derive(Debug, Clone)]
pub(crate) struct CpuWitnessGenerator {
    app_bin_path: PathBuf,
}

impl CpuWitnessGenerator {
    pub fn new(app_bin_path: PathBuf) -> Self {
        Self { app_bin_path }
    }

    pub fn generate_witness(&self, oracle: ZkEENonDeterminismSource) -> anyhow::Result<Vec<u32>> {
        let copy_source = ReadWitnessSource::new(oracle);
        let items = copy_source.get_read_items();

        let output = zksync_os_runner::run(self.app_bin_path.clone(), None, 1 << 36, copy_source);
        if output == [0u32; 8] {
            bail!("zksync_os_runner failed to execute the block");
        }

        let witness = items.borrow().clone();
        Ok(witness)
    }
}
