use std::cell::RefCell;
use std::io::Read;

use console_error_panic_hook::set_once as set_panic_hook;
use wasm_bindgen::prelude::*;

mod proof_format;
mod unified_verifier;

use proof_format::decode_proof_payload;
use unified_verifier::{
    verify_proof_in_unified_layer, CompiledCircuitsSet, UnrolledProgramProof, UnrolledProgramSetup,
};

const DEFAULT_SECURITY_80_SETUP_BIN: &[u8] =
    include_bytes!("../../../artifacts/recursion_unified_security_80_setup.bin");
const DEFAULT_SECURITY_80_LAYOUT_BIN: &[u8] =
    include_bytes!("../../../artifacts/recursion_unified_security_80_layouts.bin");
const DEFAULT_SECURITY_100_SETUP_BIN: &[u8] =
    include_bytes!("../../../artifacts/recursion_unified_security_100_setup.bin");
const DEFAULT_SECURITY_100_LAYOUT_BIN: &[u8] =
    include_bytes!("../../../artifacts/recursion_unified_security_100_layouts.bin");

struct VerifierContext {
    security_80: Option<SecurityVerifierContext>,
    security_100: Option<SecurityVerifierContext>,
}

struct SecurityVerifierContext {
    setup: UnrolledProgramSetup,
    layout: CompiledCircuitsSet,
}

#[wasm_bindgen]
#[derive(Clone, Copy, Debug)]
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
    fn defaults() -> Result<Self, String> {
        Ok(Self {
            security_80: Some(SecurityVerifierContext::parse(
                DEFAULT_SECURITY_80_SETUP_BIN,
                DEFAULT_SECURITY_80_LAYOUT_BIN,
                "security_80",
            )?),
            security_100: Some(SecurityVerifierContext::parse(
                DEFAULT_SECURITY_100_SETUP_BIN,
                DEFAULT_SECURITY_100_LAYOUT_BIN,
                "security_100",
            )?),
        })
    }

    fn single_security(
        security: SecurityLevel,
        setup_bin: &[u8],
        layout_bin: &[u8],
    ) -> Result<Self, String> {
        let parsed = SecurityVerifierContext::parse(setup_bin, layout_bin, security.label())?;
        match security {
            SecurityLevel::Security80 => Ok(Self {
                security_80: Some(parsed),
                security_100: None,
            }),
            SecurityLevel::Security100 => Ok(Self {
                security_80: None,
                security_100: Some(parsed),
            }),
        }
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

    fn set_global(self) {
        CONTEXT.with(|slot| {
            slot.borrow_mut().replace(self);
        });
    }
}

impl SecurityVerifierContext {
    fn parse(setup_bin: &[u8], layout_bin: &[u8], label: &str) -> Result<Self, String> {
        let setup = decode_exact::<UnrolledProgramSetup>(setup_bin, &format!("{label} setup"))?;
        let layout = decode_exact::<CompiledCircuitsSet>(layout_bin, &format!("{label} layouts"))?;
        Ok(Self { setup, layout })
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

    fn from_proof_wire_value(value: u8) -> Result<Self, String> {
        match value {
            80 => Ok(Self::Security80),
            100 => Ok(Self::Security100),
            _ => Err(format!("unsupported proof security level {value}")),
        }
    }
}

thread_local! {
    static CONTEXT: RefCell<Option<VerifierContext>> = const { RefCell::new(None) };
}

#[wasm_bindgen]
pub fn init_defaults() -> Result<(), JsValue> {
    set_panic_hook();
    let context = VerifierContext::defaults().map_err(|err| JsValue::from_str(&err))?;
    context.set_global();
    Ok(())
}

#[wasm_bindgen]
pub fn init_with(setup_bin: &[u8], layout_bin: &[u8]) -> Result<(), JsValue> {
    init_with_security(setup_bin, layout_bin, SecurityLevel::Security80)
}

#[wasm_bindgen]
pub fn init_with_security(
    setup_bin: &[u8],
    layout_bin: &[u8],
    security: SecurityLevel,
) -> Result<(), JsValue> {
    set_panic_hook();
    let context = VerifierContext::single_security(security, setup_bin, layout_bin)
        .map_err(|err| JsValue::from_str(&err))?;
    context.set_global();
    Ok(())
}

#[wasm_bindgen]
pub fn init_with_all(
    security_80_setup_bin: &[u8],
    security_80_layout_bin: &[u8],
    security_100_setup_bin: &[u8],
    security_100_layout_bin: &[u8],
) -> Result<(), JsValue> {
    set_panic_hook();
    let context = VerifierContext {
        security_80: Some(
            SecurityVerifierContext::parse(
                security_80_setup_bin,
                security_80_layout_bin,
                "security_80",
            )
            .map_err(|err| JsValue::from_str(&err))?,
        ),
        security_100: Some(
            SecurityVerifierContext::parse(
                security_100_setup_bin,
                security_100_layout_bin,
                "security_100",
            )
            .map_err(|err| JsValue::from_str(&err))?,
        ),
    };
    context.set_global();
    Ok(())
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

#[wasm_bindgen]
pub fn verify_proof(handle: &ProofHandle) -> VerifyResult {
    verify_proof_with_security(handle, handle.security)
}

#[wasm_bindgen]
pub fn verify_proof_with_security(handle: &ProofHandle, security: SecurityLevel) -> VerifyResult {
    CONTEXT.with(|slot| {
        let context = slot.borrow();
        let Some(context) = context.as_ref() else {
            return VerifyResult {
                success: false,
                error: Some(
                    "verifier not initialized (call init_defaults or init_with)".to_string(),
                ),
            };
        };
        let context = match context.get(security) {
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
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_include_both_security_levels() {
        let context = VerifierContext::defaults().expect("parse bundled verifier artifacts");

        assert!(context.security_80.is_some());
        assert!(context.security_100.is_some());
    }

    #[test]
    fn single_security_initializes_only_requested_level() {
        let context = VerifierContext::single_security(
            SecurityLevel::Security100,
            DEFAULT_SECURITY_100_SETUP_BIN,
            DEFAULT_SECURITY_100_LAYOUT_BIN,
        )
        .expect("parse 100-bit verifier artifacts");

        assert!(context.security_80.is_none());
        assert!(context.security_100.is_some());
    }
}
