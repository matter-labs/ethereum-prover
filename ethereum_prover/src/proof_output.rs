use std::{io::Write as _, path::PathBuf};

use anyhow::Context as _;
use flate2::{Compression, write::GzEncoder};

use crate::types::ProofSecurity;

/// Compresses verifier proof bytes into the binary format consumed by manual
/// verifier tools.
///
/// EthProofs wraps the same gzip payload in base64 for HTTP transport, but the
/// persisted file intentionally stops at gzip so it can be passed directly to
/// the JS/WASM verifier demo.
pub(crate) fn gzip_proof_bytes(proof_bytes: &[u8]) -> anyhow::Result<Vec<u8>> {
    let mut encoder = GzEncoder::new(Vec::new(), Compression::best());
    encoder
        .write_all(proof_bytes)
        .context("failed to write proof bytes into gzip encoder")?;
    encoder.finish().context("failed to finish gzip encoding")
}

#[derive(Debug, Clone)]
pub(crate) struct ProofOutput {
    output_dir: PathBuf,
}

impl ProofOutput {
    pub(crate) fn new(output_dir: PathBuf) -> Self {
        Self { output_dir }
    }

    pub(crate) fn save_gzipped_proof(
        &self,
        block_number: u64,
        security: ProofSecurity,
        proof_bytes: &[u8],
    ) -> anyhow::Result<PathBuf> {
        let block_dir = self.output_dir.join(block_number.to_string());
        std::fs::create_dir_all(&block_dir).with_context(|| {
            format!(
                "failed to create proof output directory {}",
                block_dir.display()
            )
        })?;

        let path = block_dir.join(proof_file_name(security));
        let gzipped_proof = gzip_proof_bytes(proof_bytes).with_context(|| {
            format!(
                "failed to gzip proof bytes for block {block_number} ({})",
                security.label()
            )
        })?;
        std::fs::write(&path, gzipped_proof)
            .with_context(|| format!("failed to write proof file {}", path.display()))?;
        Ok(path)
    }
}

fn proof_file_name(security: ProofSecurity) -> &'static str {
    match security {
        ProofSecurity::Security80 => "proof_80.bin",
        ProofSecurity::Security100 => "proof_100.bin",
    }
}

impl ProofSecurity {
    fn label(self) -> &'static str {
        match self {
            Self::Security80 => "80-bit",
            Self::Security100 => "100-bit",
        }
    }
}

#[cfg(test)]
mod tests {
    use std::io::Read as _;

    use flate2::read::GzDecoder;

    use super::*;

    #[test]
    fn gzip_proof_bytes_roundtrips() {
        let proof_bytes = b"proof-bytes-test-vector";

        let gzipped = gzip_proof_bytes(proof_bytes).expect("gzip proof bytes");

        let mut decoder = GzDecoder::new(gzipped.as_slice());
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .expect("decode gzipped proof bytes");
        assert_eq!(decoded, proof_bytes);
    }

    #[test]
    fn proof_output_writes_security_specific_gzip_file() {
        let temp_dir = tempfile::tempdir().expect("create temp dir");
        let output = ProofOutput::new(temp_dir.path().to_path_buf());
        let proof_bytes = b"manual-test-proof";

        let path = output
            .save_gzipped_proof(42, ProofSecurity::Security100, proof_bytes)
            .expect("save proof");

        assert_eq!(path, temp_dir.path().join("42").join("proof_100.bin"));
        let gzipped = std::fs::read(path).expect("read saved proof");
        let mut decoder = GzDecoder::new(gzipped.as_slice());
        let mut decoded = Vec::new();
        decoder
            .read_to_end(&mut decoded)
            .expect("decode saved proof");
        assert_eq!(decoded, proof_bytes);
    }
}
