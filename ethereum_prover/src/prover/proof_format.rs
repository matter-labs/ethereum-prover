use execution_utils::unrolled::UnrolledProgramProof;

use crate::types::ProofSecurity;

pub(crate) const PROOF_MAGIC: [u8; 8] = *b"EPROOF01";
const PROOF_FORMAT_VERSION: u8 = 1;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct EncodedProof<P> {
    magic: [u8; 8],
    version: u8,
    security: u8,
    proof: P,
}

fn encode_envelope<P: serde::Serialize>(
    proof: P,
    security: ProofSecurity,
) -> Result<Vec<u8>, bincode::error::EncodeError> {
    // The outer EthProofs transport still handles gzip + base64. This envelope
    // is only the inner bincode payload, and starts with a fixed magic so
    // verifiers can distinguish it from legacy raw Airbender proofs.
    let encoded = EncodedProof {
        magic: PROOF_MAGIC,
        version: PROOF_FORMAT_VERSION,
        security: security.proof_wire_value(),
        proof,
    };
    bincode::serde::encode_to_vec(&encoded, bincode::config::standard())
}

pub(crate) fn encode_proof(
    proof: UnrolledProgramProof,
    security: ProofSecurity,
) -> Result<Vec<u8>, bincode::error::EncodeError> {
    encode_envelope(proof, security)
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECURITY_100_UNIT_ENVELOPE_HEX: &str =
        include_str!("../../../test_fixtures/proof_format/security_100_unit_envelope.hex");

    #[test]
    fn encoded_payload_starts_with_magic() {
        let encoded = EncodedProof {
            magic: PROOF_MAGIC,
            version: PROOF_FORMAT_VERSION,
            security: ProofSecurity::Security100.proof_wire_value(),
            proof: (),
        };

        let bytes = bincode::serde::encode_to_vec(&encoded, bincode::config::standard())
            .expect("encode test envelope");

        assert!(bytes.starts_with(&PROOF_MAGIC));
    }

    #[test]
    fn security_100_unit_envelope_matches_golden_vector() {
        // The proof field is intentionally `()` here. The contract under test is
        // the shallow envelope shared with proof_verifier_js, not Airbender's
        // internal proof schema or verifier semantics.
        let bytes =
            encode_envelope((), ProofSecurity::Security100).expect("encode unit proof envelope");

        assert_eq!(to_hex(&bytes), SECURITY_100_UNIT_ENVELOPE_HEX.trim());
    }

    fn to_hex(bytes: &[u8]) -> String {
        const HEX: &[u8; 16] = b"0123456789abcdef";
        let mut out = String::with_capacity(bytes.len() * 2);
        for byte in bytes {
            out.push(HEX[(byte >> 4) as usize] as char);
            out.push(HEX[(byte & 0x0f) as usize] as char);
        }
        out
    }
}
