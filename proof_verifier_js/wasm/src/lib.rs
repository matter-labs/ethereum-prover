use std::io::Read;

use console_error_panic_hook::set_once as set_panic_hook;
use wasm_bindgen::prelude::*;

mod proof_format;
mod unified_verifier;
mod verification_key_format;

use proof_format::decode_proof_payload;
use unified_verifier::{
    verify_proof_in_unified_layer, CompiledCircuitsSet, UnrolledProgramProof, UnrolledProgramSetup,
};
use verification_key_format::decode_verification_key;

struct VerifierContext {
    security: SecurityLevel,
    setup: UnrolledProgramSetup,
    layout: CompiledCircuitsSet,
}

#[wasm_bindgen]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum SecurityLevel {
    Security80,
    Security100,
}

fn decode_exact<T: serde::de::DeserializeOwned>(bytes: &[u8], what: &str) -> Result<T, String> {
    let (value, bytes_read): (T, usize) =
        bincode::serde::decode_from_slice(bytes, bincode::config::standard())
            .map_err(|err| format!("failed to parse {what}: {err}"))?;

    if bytes_read != bytes.len() {
        return Err(format!(
            "failed to parse {what}: trailing {} byte(s) indicate an incompatible format",
            bytes.len() - bytes_read
        ));
    }

    Ok(value)
}

impl VerifierContext {
    fn from_key(vk_bin: &[u8]) -> Result<Self, String> {
        let decoded = decode_verification_key::<UnrolledProgramSetup, CompiledCircuitsSet>(vk_bin)
            .map_err(|err| format!("failed to parse verification key: {err}"))?;
        Ok(Self {
            security: decoded.security,
            setup: decoded.setup,
            layout: decoded.layouts,
        })
    }

    fn from_legacy_key(setup_bin: &[u8], layout_bin: &[u8]) -> Result<Self, String> {
        // Legacy split keys predate security-tagged proof/VK envelopes and are
        // only kept for deployed 80-bit artifacts. New integrations should use
        // the single-file VK format, which carries its security level explicitly.
        let setup = decode_exact::<UnrolledProgramSetup>(
            setup_bin,
            &format!("{} setup", SecurityLevel::Security80.label()),
        )?;
        let layout = decode_exact::<CompiledCircuitsSet>(
            layout_bin,
            &format!("{} layouts", SecurityLevel::Security80.label()),
        )?;
        Ok(Self {
            security: SecurityLevel::Security80,
            setup,
            layout,
        })
    }

    fn ensure_security_matches(&self, proof_security: SecurityLevel) -> Result<(), String> {
        if self.security != proof_security {
            return Err(format!(
                "verification key is for {} security but proof is for {} security",
                self.security.error_label(),
                proof_security.error_label()
            ));
        }
        Ok(())
    }
}

impl SecurityLevel {
    fn airbender_security_model(self) -> verifier_common::SecurityModel {
        match self {
            Self::Security80 => verifier_common::SecurityModel::Security80,
            Self::Security100 => verifier_common::SecurityModel::Security100,
        }
    }

    fn label(self) -> &'static str {
        match self {
            Self::Security80 => "security_80",
            Self::Security100 => "security_100",
        }
    }

    fn error_label(self) -> &'static str {
        match self {
            Self::Security80 => "80-bit",
            Self::Security100 => "100-bit",
        }
    }

    pub(crate) fn from_wire_value(value: u8) -> Result<Self, String> {
        match value {
            80 => Ok(Self::Security80),
            100 => Ok(Self::Security100),
            _ => Err(format!("unsupported security level {value}")),
        }
    }
}

#[wasm_bindgen]
pub struct WasmVerifier {
    context: VerifierContext,
}

#[wasm_bindgen]
impl WasmVerifier {
    #[wasm_bindgen(js_name = fromKey)]
    pub fn from_key(vk_bin: &[u8]) -> Result<Self, JsValue> {
        set_panic_hook();
        let context = VerifierContext::from_key(vk_bin).map_err(|err| JsValue::from_str(&err))?;
        Ok(Self { context })
    }

    #[wasm_bindgen(js_name = fromLegacyKey)]
    pub fn from_legacy_key(setup_bin: &[u8], layout_bin: &[u8]) -> Result<Self, JsValue> {
        set_panic_hook();
        let context = VerifierContext::from_legacy_key(setup_bin, layout_bin)
            .map_err(|err| JsValue::from_str(&err))?;
        Ok(Self { context })
    }

    #[wasm_bindgen(js_name = verifyProof)]
    pub fn verify_proof(&self, handle: &ProofHandle) -> VerifyResult {
        if let Err(err) = self.context.ensure_security_matches(handle.security) {
            return VerifyResult {
                success: false,
                error: Some(err),
            };
        }

        match verify_proof_in_unified_layer(
            &handle.proof,
            &self.context.setup,
            &self.context.layout,
            self.context.security.airbender_security_model(),
            false,
        ) {
            Ok(_result) => VerifyResult {
                success: true,
                error: None,
            },
            Err(()) => VerifyResult {
                success: false,
                error: Some("Failed to verify proof".to_string()),
            },
        }
    }
}

#[wasm_bindgen]
pub struct ProofHandle {
    proof: UnrolledProgramProof,
    security: SecurityLevel,
}

#[wasm_bindgen]
pub fn deserialize_proof_bytes(proof_bytes: &[u8]) -> Result<ProofHandle, JsValue> {
    let mut decoder = flate2::read::GzDecoder::new(proof_bytes);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|err| JsValue::from_str(&format!("gzip decode failed: {err}")))?;

    let decoded = decode_proof_payload::<UnrolledProgramProof>(&decompressed)
        .map_err(|err| JsValue::from_str(&err))?;

    Ok(ProofHandle {
        proof: decoded.proof,
        security: decoded.security,
    })
}

#[wasm_bindgen]
pub struct VerifyResult {
    success: bool,
    error: Option<String>,
}

#[wasm_bindgen]
impl VerifyResult {
    #[wasm_bindgen(getter)]
    pub fn success(&self) -> bool {
        self.success
    }

    #[wasm_bindgen]
    pub fn error(&self) -> Option<JsValue> {
        self.error.as_ref().map(|e| JsValue::from_str(e))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECURITY_100_VK_BIN: &[u8] =
        include_bytes!("../../../artifacts/recursion_unified_security_100.vk.bin");
    const LEGACY_SECURITY_80_SETUP_FOR_TESTS_BIN: &[u8] = include_bytes!(
        "../../../test_fixtures/verification_key_format/recursion_unified_security_80_setup_for_tests.bin"
    );
    const LEGACY_SECURITY_80_LAYOUTS_FOR_TESTS_BIN: &[u8] = include_bytes!(
        "../../../test_fixtures/verification_key_format/recursion_unified_security_80_layouts_for_tests.bin"
    );

    #[test]
    fn unified_key_keeps_declared_security_level() {
        let context = VerifierContext::from_key(SECURITY_100_VK_BIN)
            .expect("parse bundled test verification key");

        assert_eq!(context.security, SecurityLevel::Security100);
    }

    #[test]
    fn security_mismatch_error_is_explicit() {
        let context = VerifierContext::from_key(SECURITY_100_VK_BIN)
            .expect("parse bundled test verification key");
        let err = context
            .ensure_security_matches(SecurityLevel::Security80)
            .expect_err("mismatched proof security should be rejected before verification");

        assert_eq!(
            err,
            "verification key is for 100-bit security but proof is for 80-bit security"
        );
    }

    #[test]
    fn legacy_split_key_still_initializes_80_bit_context() {
        let context = VerifierContext::from_legacy_key(
            LEGACY_SECURITY_80_SETUP_FOR_TESTS_BIN,
            LEGACY_SECURITY_80_LAYOUTS_FOR_TESTS_BIN,
        )
        .expect("parse legacy 80-bit split verification key");

        assert_eq!(context.security, SecurityLevel::Security80);
    }
}
