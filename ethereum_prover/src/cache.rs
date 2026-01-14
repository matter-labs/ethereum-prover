use std::path::PathBuf;

use alloy::rpc::types::{Block as RpcBlock, debug::ExecutionWitness};

#[derive(Debug, Clone)]
pub(super) struct CacheStorage {
    root: PathBuf,
}

#[derive(Debug, Clone)]
struct BlockCachePaths {
    dir: PathBuf,
    block_json: PathBuf,
    execution_witness_json: PathBuf,
}

impl CacheStorage {
    pub fn new(root: impl Into<PathBuf>) -> Self {
        let this = Self { root: root.into() };
        this.ensure_root()
            .expect("Failed to create cache root directory");
        this
    }

    fn ensure_root(&self) -> anyhow::Result<()> {
        std::fs::create_dir_all(&self.root)?;
        Ok(())
    }

    pub fn has_cached_block(&self, block_number: u64) -> bool {
        let paths = self.block_paths(block_number);
        paths.block_json.exists() && paths.execution_witness_json.exists()
    }

    pub fn cache_block(
        &self,
        block_number: u64,
        block: &RpcBlock,
        execution_witness: &ExecutionWitness,
    ) -> anyhow::Result<()> {
        self.write_rpc_block(block_number, block)?;
        self.write_execution_witness(block_number, execution_witness)?;
        Ok(())
    }

    pub fn remove_cached_block(&self, block_number: u64) -> anyhow::Result<()> {
        let paths = self.block_paths(block_number);
        if paths.dir.exists() {
            std::fs::remove_dir_all(paths.dir)?;
        }
        Ok(())
    }

    pub fn load_block(
        &self,
        block_number: u64,
    ) -> anyhow::Result<Option<(RpcBlock, ExecutionWitness)>> {
        let block = match self.load_rpc_block(block_number)? {
            Some(b) => b,
            None => return Ok(None),
        };
        let witness = match self.load_execution_witness(block_number)? {
            Some(w) => w,
            None => return Ok(None),
        };
        Ok(Some((block, witness)))
    }

    fn block_paths(&self, block_number: u64) -> BlockCachePaths {
        let dir = self.root.join("blocks").join(block_number.to_string());
        BlockCachePaths {
            dir: dir.clone(),
            block_json: dir.join("block.json"),
            execution_witness_json: dir.join("execution_witness.json"),
        }
    }

    fn ensure_block_dir(&self, block_number: u64) -> anyhow::Result<BlockCachePaths> {
        let paths = self.block_paths(block_number);
        std::fs::create_dir_all(&paths.dir)?;
        Ok(paths)
    }

    fn load_rpc_block(&self, block_number: u64) -> anyhow::Result<Option<RpcBlock>> {
        let paths = self.block_paths(block_number);
        if !paths.block_json.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(paths.block_json)?;
        let block = serde_json::from_str(&data)?;
        Ok(Some(block))
    }

    fn write_rpc_block(&self, block_number: u64, block: &RpcBlock) -> anyhow::Result<()> {
        let paths = self.ensure_block_dir(block_number)?;
        let data = serde_json::to_string_pretty(block)?;
        std::fs::write(paths.block_json, data)?;
        Ok(())
    }

    fn load_execution_witness(
        &self,
        block_number: u64,
    ) -> anyhow::Result<Option<ExecutionWitness>> {
        let paths = self.block_paths(block_number);
        if !paths.execution_witness_json.exists() {
            return Ok(None);
        }
        let data = std::fs::read_to_string(paths.execution_witness_json)?;
        let witness = serde_json::from_str(&data)?;
        Ok(Some(witness))
    }

    fn write_execution_witness(
        &self,
        block_number: u64,
        witness: &ExecutionWitness,
    ) -> anyhow::Result<()> {
        let paths = self.ensure_block_dir(block_number)?;
        let data = serde_json::to_string_pretty(witness)?;
        std::fs::write(paths.execution_witness_json, data)?;
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::{CacheStorage, RpcBlock};
    use tempfile::tempdir;

    #[test]
    fn cache_roundtrips_block_and_execution_witness() {
        let dir = tempdir().expect("create tempdir");
        let cache = CacheStorage::new(dir.path());
        let block_number = 123_u64;

        let block = RpcBlock {
            header: alloy::rpc::types::Header {
                inner: alloy::consensus::Header {
                    number: block_number,
                    ..Default::default()
                },
                ..Default::default()
            },
            uncles: Vec::new(),
            transactions: alloy::rpc::types::BlockTransactions::Hashes(Vec::new()),
            withdrawals: None,
        };

        cache
            .write_rpc_block(block_number, &block)
            .expect("write block");
        let loaded_block = cache
            .load_rpc_block(block_number)
            .expect("read block")
            .expect("block exists");
        assert_eq!(loaded_block.header.number, block_number);

        let witness = alloy::rpc::types::debug::ExecutionWitness::default();
        cache
            .write_execution_witness(block_number, &witness)
            .expect("write witness");
        let loaded = cache
            .load_execution_witness(block_number)
            .expect("read witness")
            .expect("witness exists");
        assert_eq!(loaded.headers.len(), witness.headers.len());
    }
}
