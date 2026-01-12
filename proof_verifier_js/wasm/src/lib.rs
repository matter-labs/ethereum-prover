use std::cell::RefCell;
use std::io::Read;

use console_error_panic_hook::set_once as set_panic_hook;
use wasm_bindgen::prelude::*;

mod unified_verifier;

use unified_verifier::{
    verify_proof_in_unified_layer, CompiledCircuitsSet, UnrolledProgramProof, UnrolledProgramSetup,
};

const DEFAULT_SETUP_BIN: &[u8] = include_bytes!("../../../artifacts/recursion_unified_setup.bin");
const DEFAULT_LAYOUT_BIN: &[u8] =
    include_bytes!("../../../artifacts/recursion_unified_layouts.bin");

struct VerifierContext {
    setup: UnrolledProgramSetup,
    layout: CompiledCircuitsSet,
}

impl VerifierContext {
    fn parse(setup_bin: &[u8], layout_bin: &[u8]) -> Result<Self, String> {
        let (setup, _): (UnrolledProgramSetup, usize) =
            bincode::serde::decode_from_slice(setup_bin, bincode::config::standard())
                .map_err(|err| format!("failed to parse setup.bin: {err}"))?;
        let (layout, _): (CompiledCircuitsSet, usize) =
            bincode::serde::decode_from_slice(layout_bin, bincode::config::standard())
                .map_err(|err| format!("failed to parse layouts.bin: {err}"))?;
        Ok(Self { setup, layout })
    }

    fn set_global(self) {
        CONTEXT.with(|slot| {
            slot.borrow_mut().replace(self);
        });
    }
}

thread_local! {
    static CONTEXT: RefCell<Option<VerifierContext>> = const { RefCell::new(None) };
}

#[wasm_bindgen]
pub fn init_defaults() -> Result<(), JsValue> {
    init_with(DEFAULT_SETUP_BIN, DEFAULT_LAYOUT_BIN)
}

#[wasm_bindgen]
pub fn init_with(setup_bin: &[u8], layout_bin: &[u8]) -> Result<(), JsValue> {
    set_panic_hook();
    let context =
        VerifierContext::parse(setup_bin, layout_bin).map_err(|err| JsValue::from_str(&err))?;
    context.set_global();
    Ok(())
}

#[wasm_bindgen]
pub struct ProofHandle {
    proof: UnrolledProgramProof,
}

#[wasm_bindgen]
pub fn deserialize_proof_bytes(proof_bytes: &[u8]) -> Result<ProofHandle, JsValue> {
    let mut decoder = flate2::read::GzDecoder::new(proof_bytes);
    let mut decompressed = Vec::new();
    decoder
        .read_to_end(&mut decompressed)
        .map_err(|err| JsValue::from_str(&format!("gzip decode failed: {err}")))?;

    let (proof, _bytes_read): (UnrolledProgramProof, usize) =
        bincode::serde::decode_from_slice(&decompressed, bincode::config::standard())
            .map_err(|err| JsValue::from_str(&format!("bincode decode failed: {err}")))?;

    Ok(ProofHandle { proof })
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

        match verify_proof_in_unified_layer(&handle.proof, &context.setup, &context.layout, false) {
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
