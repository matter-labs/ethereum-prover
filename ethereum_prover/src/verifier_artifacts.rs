use std::path::{Path, PathBuf};

use airbender_riscv_transpiler::cycle::IWithoutByteAccessIsaConfigWithDelegation;
use anyhow::Context as _;
use execution_utils::{
    RecursionArtifact, RecursionLayer, setups,
    unified_circuit::compute_unified_setup_for_machine_configuration, verifier_binaries,
};

use crate::types::ProofSecurity;

const LEGACY_SETUP_FILENAME: &str = "recursion_unified_setup.bin";
const LEGACY_LAYOUTS_FILENAME: &str = "recursion_unified_layouts.bin";

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
    write_bincode(&paths.setup, &setup, "setup")?;
    write_bincode(&paths.layouts, &layouts, "layouts")?;

    if matches!(security, ProofSecurity::Security80) {
        // Legacy consumers still look for the unsuffixed names. Keep them as
        // 80-bit aliases until all callers switch to explicit security labels.
        write_bincode(
            &output_dir.join(LEGACY_SETUP_FILENAME),
            &setup,
            "legacy setup",
        )?;
        write_bincode(
            &output_dir.join(LEGACY_LAYOUTS_FILENAME),
            &layouts,
            "legacy layouts",
        )?;
    }

    tracing::info!(
        "Generated {} verifier artifacts in {}",
        security.name(),
        output_dir.display()
    );
    Ok(())
}

fn write_bincode<T: serde::Serialize>(path: &Path, value: &T, what: &str) -> anyhow::Result<()> {
    let bytes = bincode::serde::encode_to_vec(value, bincode::config::standard())
        .with_context(|| format!("failed to encode {what} artifact"))?;
    std::fs::write(path, bytes)
        .with_context(|| format!("failed to write {what} artifact {}", path.display()))
}

struct ArtifactPaths {
    setup: PathBuf,
    layouts: PathBuf,
}

impl ArtifactPaths {
    fn new(output_dir: &Path, security: ProofSecurity) -> Self {
        Self {
            setup: output_dir.join(format!(
                "recursion_unified_{}_setup.bin",
                security.file_label()
            )),
            layouts: output_dir.join(format!(
                "recursion_unified_{}_layouts.bin",
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
            paths.setup,
            PathBuf::from("artifacts/recursion_unified_security_100_setup.bin")
        );
        assert_eq!(
            paths.layouts,
            PathBuf::from("artifacts/recursion_unified_security_100_layouts.bin")
        );
    }
}
