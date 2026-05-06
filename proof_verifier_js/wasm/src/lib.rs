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
    security_80: Option<SecurityVerifierContext>,
    security_100: Option<SecurityVerifierContext>,
}

struct SecurityVerifierContext {
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
    fn empty() -> Self {
        Self {
            security_80: None,
            security_100: None,
        }
    }

    fn from_key(vk_bin: &[u8]) -> Result<Self, String> {
        let mut context = Self::empty();
        let parsed = SecurityVerifierContext::parse_key(vk_bin, "verification key", None)?;
        context.insert(parsed.security, parsed.context)?;
        Ok(context)
    }

    fn from_key_for_security(vk_bin: &[u8], security: SecurityLevel) -> Result<Self, String> {
        let mut context = Self::empty();
        let parsed = SecurityVerifierContext::parse_key(vk_bin, security.label(), Some(security))?;
        context.insert(security, parsed.context)?;
        Ok(context)
    }

    fn from_keys(security_80_vk_bin: &[u8], security_100_vk_bin: &[u8]) -> Result<Self, String> {
        let mut context = Self::empty();
        let security_80 = SecurityVerifierContext::parse_key(
            security_80_vk_bin,
            SecurityLevel::Security80.label(),
            Some(SecurityLevel::Security80),
        )?;
        let security_100 = SecurityVerifierContext::parse_key(
            security_100_vk_bin,
            SecurityLevel::Security100.label(),
            Some(SecurityLevel::Security100),
        )?;
        context.insert(SecurityLevel::Security80, security_80.context)?;
        context.insert(SecurityLevel::Security100, security_100.context)?;
        Ok(context)
    }

    fn from_legacy_key(
        security: SecurityLevel,
        setup_bin: &[u8],
        layout_bin: &[u8],
    ) -> Result<Self, String> {
        let mut context = Self::empty();
        let parsed =
            SecurityVerifierContext::parse_legacy(setup_bin, layout_bin, security.label())?;
        context.insert(security, parsed)?;
        Ok(context)
    }

    fn from_legacy_keys(
        security_80_setup_bin: &[u8],
        security_80_layout_bin: &[u8],
        security_100_setup_bin: &[u8],
        security_100_layout_bin: &[u8],
    ) -> Result<Self, String> {
        let mut context = Self::empty();
        let security_80 = SecurityVerifierContext::parse_legacy(
            security_80_setup_bin,
            security_80_layout_bin,
            SecurityLevel::Security80.label(),
        )?;
        let security_100 = SecurityVerifierContext::parse_legacy(
            security_100_setup_bin,
            security_100_layout_bin,
            SecurityLevel::Security100.label(),
        )?;
        context.insert(SecurityLevel::Security80, security_80)?;
        context.insert(SecurityLevel::Security100, security_100)?;
        Ok(context)
    }

    fn get(&self, security: SecurityLevel) -> Result<&SecurityVerifierContext, String> {
        let context = match security {
            SecurityLevel::Security80 => self.security_80.as_ref(),
            SecurityLevel::Security100 => self.security_100.as_ref(),
        };
        context.ok_or_else(|| {
            format!(
                "{} verifier artifacts are not initialized",
                security.error_label()
            )
        })
    }

    fn insert(
        &mut self,
        security: SecurityLevel,
        context: SecurityVerifierContext,
    ) -> Result<(), String> {
        let slot = match security {
            SecurityLevel::Security80 => &mut self.security_80,
            SecurityLevel::Security100 => &mut self.security_100,
        };
        if slot.is_some() {
            return Err(format!(
                "{} verifier artifacts were provided more than once",
                security.error_label()
            ));
        }
        *slot = Some(context);
        Ok(())
    }
}

impl SecurityVerifierContext {
    fn parse_legacy(setup_bin: &[u8], layout_bin: &[u8], label: &str) -> Result<Self, String> {
        let setup = decode_exact::<UnrolledProgramSetup>(setup_bin, &format!("{label} setup"))?;
        let layout = decode_exact::<CompiledCircuitsSet>(layout_bin, &format!("{label} layouts"))?;
        Ok(Self { setup, layout })
    }

    fn parse_key(
        vk_bin: &[u8],
        label: &str,
        expected_security: Option<SecurityLevel>,
    ) -> Result<ParsedSecurityVerifierContext, String> {
        let decoded = decode_verification_key::<UnrolledProgramSetup, CompiledCircuitsSet>(vk_bin)
            .map_err(|err| format!("failed to parse {label} verification key: {err}"))?;
        if let Some(expected_security) = expected_security {
            if decoded.security != expected_security {
                return Err(format!(
                    "{label} verification key declares {}, expected {}",
                    decoded.security.error_label(),
                    expected_security.error_label()
                ));
            }
        }

        Ok(ParsedSecurityVerifierContext {
            security: decoded.security,
            context: Self {
                setup: decoded.setup,
                layout: decoded.layouts,
            },
        })
    }
}

struct ParsedSecurityVerifierContext {
    security: SecurityLevel,
    context: SecurityVerifierContext,
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

    #[wasm_bindgen(js_name = fromKeyForSecurity)]
    pub fn from_key_for_security(vk_bin: &[u8], security: SecurityLevel) -> Result<Self, JsValue> {
        set_panic_hook();
        let context = VerifierContext::from_key_for_security(vk_bin, security)
            .map_err(|err| JsValue::from_str(&err))?;
        Ok(Self { context })
    }

    #[wasm_bindgen(js_name = fromKeys)]
    pub fn from_keys(
        security_80_vk_bin: &[u8],
        security_100_vk_bin: &[u8],
    ) -> Result<Self, JsValue> {
        set_panic_hook();
        let context = VerifierContext::from_keys(security_80_vk_bin, security_100_vk_bin)
            .map_err(|err| JsValue::from_str(&err))?;
        Ok(Self { context })
    }

    #[wasm_bindgen(js_name = fromLegacyKey)]
    pub fn from_legacy_key(
        setup_bin: &[u8],
        layout_bin: &[u8],
        security: SecurityLevel,
    ) -> Result<Self, JsValue> {
        set_panic_hook();
        let context = VerifierContext::from_legacy_key(security, setup_bin, layout_bin)
            .map_err(|err| JsValue::from_str(&err))?;
        Ok(Self { context })
    }

    #[wasm_bindgen(js_name = fromLegacyKeys)]
    pub fn from_legacy_keys(
        security_80_setup_bin: &[u8],
        security_80_layout_bin: &[u8],
        security_100_setup_bin: &[u8],
        security_100_layout_bin: &[u8],
    ) -> Result<Self, JsValue> {
        set_panic_hook();
        let context = VerifierContext::from_legacy_keys(
            security_80_setup_bin,
            security_80_layout_bin,
            security_100_setup_bin,
            security_100_layout_bin,
        )
        .map_err(|err| JsValue::from_str(&err))?;
        Ok(Self { context })
    }

    #[wasm_bindgen(js_name = verifyProof)]
    pub fn verify_proof(&self, handle: &ProofHandle) -> VerifyResult {
        self.verify_proof_with_security(handle, handle.security)
    }

    #[wasm_bindgen(js_name = verifyProofWithSecurity)]
    pub fn verify_proof_with_security(
        &self,
        handle: &ProofHandle,
        security: SecurityLevel,
    ) -> VerifyResult {
        let context = match self.context.get(security) {
            Ok(context) => context,
            Err(err) => {
                return VerifyResult {
                    success: false,
                    error: Some(err),
                };
            }
        };

        match verify_proof_in_unified_layer(
            &handle.proof,
            &context.setup,
            &context.layout,
            security.airbender_security_model(),
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
    const SECURITY_100_SETUP_BIN: &[u8] =
        include_bytes!("../../../artifacts/recursion_unified_security_100_setup.bin");
    const SECURITY_100_LAYOUT_BIN: &[u8] =
        include_bytes!("../../../artifacts/recursion_unified_security_100_layouts.bin");

    #[test]
    fn unified_key_initializes_only_declared_security_level() {
        let context = VerifierContext::from_key(SECURITY_100_VK_BIN)
            .expect("parse bundled test verification key");

        assert!(context.security_80.is_none());
        assert!(context.security_100.is_some());
    }

    #[test]
    fn unified_key_rejects_mismatched_security_slot() {
        let err = match VerifierContext::from_key_for_security(
            SECURITY_100_VK_BIN,
            SecurityLevel::Security80,
        ) {
            Ok(_) => panic!("mismatched verification key security should be rejected"),
            Err(err) => err,
        };

        assert!(err.contains("expected 80-bit"));
    }

    #[test]
    fn legacy_split_key_still_initializes_explicit_security_level() {
        let context = VerifierContext::from_legacy_key(
            SecurityLevel::Security100,
            SECURITY_100_SETUP_BIN,
            SECURITY_100_LAYOUT_BIN,
        )
        .expect("parse legacy split verification key");

        assert!(context.security_80.is_none());
        assert!(context.security_100.is_some());
    }
}
