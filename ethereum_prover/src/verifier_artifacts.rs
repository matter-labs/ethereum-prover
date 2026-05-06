use std::path::{Path, PathBuf};

use airbender_riscv_transpiler::cycle::IWithoutByteAccessIsaConfigWithDelegation;
use anyhow::Context as _;
use execution_utils::{
    RecursionArtifact, RecursionLayer, setups,
    unified_circuit::compute_unified_setup_for_machine_configuration, verifier_binaries,
};

use crate::{types::ProofSecurity, verification_key_format::encode_verification_key};

pub fn generate_verifier_artifacts(
    output_dir: &Path,
    security: Option<ProofSecurity>,
) -> anyhow::Result<()> {
    let securities = match security {
        Some(security) => vec![security],
        None => vec![ProofSecurity::Security80, ProofSecurity::Security100],
    };

    std::fs::create_dir_all(output_dir).with_context(|| {
        format!(
            "failed to create verifier artifact output directory {}",
            output_dir.display()
        )
    })?;

    for security in securities {
        generate_artifacts_for_security(output_dir, security).with_context(|| {
            format!("failed to generate {} verifier artifacts", security.name())
        })?;
    }

    Ok(())
}

fn generate_artifacts_for_security(
    output_dir: &Path,
    security: ProofSecurity,
) -> anyhow::Result<()> {
    tracing::info!("Generating {} verifier artifacts", security.name());

    // The final proof is verified by the unified recursion program. We compute
    // setup/layout directly from the new Airbender verifier binary instead of
    // relying on stale zksync-os test outputs.
    let binary = verifier_binaries::recursion_artifact(
        security.airbender_security_model(),
        RecursionLayer::Unified,
        RecursionArtifact::Bin,
    );
    let text = verifier_binaries::recursion_artifact(
        security.airbender_security_model(),
        RecursionLayer::Unified,
        RecursionArtifact::Txt,
    );

    let mut padded_binary = binary.to_vec();
    setups::pad_bytecode_bytes_for_proving(&mut padded_binary);
    let mut padded_text = text.to_vec();
    setups::pad_bytecode_bytes_for_proving(&mut padded_text);

    let mut padded_binary_u32 = setups::binary_u8_to_u32(binary);
    setups::pad_bytecode_for_proving(&mut padded_binary_u32);

    let setup = compute_unified_setup_for_machine_configuration::<
        IWithoutByteAccessIsaConfigWithDelegation,
    >(&padded_binary, &padded_text);
    let layouts = setups::get_unified_circuit_artifact_for_machine_type::<
        IWithoutByteAccessIsaConfigWithDelegation,
    >(&padded_binary_u32);

    let paths = ArtifactPaths::new(output_dir, security);
    let verification_key = encode_verification_key(&setup, &layouts, security)
        .with_context(|| format!("failed to encode {} verification key", security.name()))?;
    write_bytes(
        &paths.verification_key,
        verification_key,
        "verification key",
    )?;

    tracing::info!(
        "Generated {} verifier artifacts in {}",
        security.name(),
        output_dir.display()
    );
    Ok(())
}

fn write_bytes(path: &Path, bytes: Vec<u8>, what: &str) -> anyhow::Result<()> {
    std::fs::write(path, bytes)
        .with_context(|| format!("failed to write {what} artifact {}", path.display()))
}

struct ArtifactPaths {
    verification_key: PathBuf,
}

impl ArtifactPaths {
    fn new(output_dir: &Path, security: ProofSecurity) -> Self {
        Self {
            verification_key: output_dir.join(format!(
                "recursion_unified_{}.vk.bin",
                security.file_label()
            )),
        }
    }
}

impl ProofSecurity {
    fn name(self) -> &'static str {
        match self {
            Self::Security80 => "80-bit",
            Self::Security100 => "100-bit",
        }
    }

    fn file_label(self) -> &'static str {
        match self {
            Self::Security80 => "security_80",
            Self::Security100 => "security_100",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_paths_include_security_label() {
        let paths = ArtifactPaths::new(Path::new("artifacts"), ProofSecurity::Security100);

        assert_eq!(
            paths.verification_key,
            PathBuf::from("artifacts/recursion_unified_security_100.vk.bin")
        );
    }
}
