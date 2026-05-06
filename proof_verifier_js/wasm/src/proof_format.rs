use serde::de::DeserializeOwned;

use crate::SecurityLevel;

pub(crate) const PROOF_MAGIC: [u8; 8] = *b"EPROOF01";
const PROOF_FORMAT_VERSION: u8 = 1;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct EncodedProof<P> {
    magic: [u8; 8],
    version: u8,
    security: u8,
    proof: P,
}

#[derive(Debug)]
pub(crate) struct DecodedProof<P> {
    pub proof: P,
    pub security: SecurityLevel,
}

pub(crate) fn decode_proof_payload<P: DeserializeOwned>(
    bytes: &[u8],
) -> Result<DecodedProof<P>, String> {
    if bytes.starts_with(&PROOF_MAGIC) {
        return decode_enveloped_proof(bytes);
    }

    let proof = crate::decode_exact::<P>(bytes, "legacy proof")?;
    Ok(DecodedProof {
        proof,
        security: SecurityLevel::Security80,
    })
}

fn decode_enveloped_proof<P: DeserializeOwned>(bytes: &[u8]) -> Result<DecodedProof<P>, String> {
    let encoded = crate::decode_exact::<EncodedProof<P>>(bytes, "proof envelope")?;
    if encoded.magic != PROOF_MAGIC {
        return Err("proof envelope magic does not match expected value".to_string());
    }
    if encoded.version != PROOF_FORMAT_VERSION {
        return Err(format!(
            "unsupported proof envelope version {}",
            encoded.version
        ));
    }
    let security = SecurityLevel::from_wire_value(encoded.security)?;
    Ok(DecodedProof {
        proof: encoded.proof,
        security,
    })
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
            security: 100,
            proof: (),
        };

        let bytes = bincode::serde::encode_to_vec(&encoded, bincode::config::standard())
            .expect("encode test envelope");

        assert!(bytes.starts_with(&PROOF_MAGIC));
    }

    #[test]
    fn security_100_golden_envelope_decodes_security_tag() {
        let bytes = decode_hex_fixture(SECURITY_100_UNIT_ENVELOPE_HEX);

        let decoded = decode_proof_payload::<()>(&bytes).expect("decode golden unit envelope");

        assert!(matches!(decoded.security, SecurityLevel::Security100));
    }

    #[test]
    fn legacy_payload_defaults_to_security_80() {
        let bytes = bincode::serde::encode_to_vec((), bincode::config::standard())
            .expect("encode legacy unit proof");

        let decoded = decode_proof_payload::<()>(&bytes).expect("decode legacy unit proof");

        assert!(matches!(decoded.security, SecurityLevel::Security80));
    }

    #[test]
    fn invalid_envelope_does_not_fall_back_to_legacy() {
        let mut bytes = PROOF_MAGIC.to_vec();
        bytes.push(0xff);

        let err = decode_proof_payload::<()>(&bytes).expect_err("reject corrupt envelope");

        assert!(err.contains("proof envelope"));
    }

    fn decode_hex_fixture(hex: &str) -> Vec<u8> {
        let trimmed = hex.trim();
        assert_eq!(
            trimmed.len() % 2,
            0,
            "hex fixtures must have an even number of digits"
        );

        trimmed
            .as_bytes()
            .chunks_exact(2)
            .map(|pair| (hex_nibble(pair[0]) << 4) | hex_nibble(pair[1]))
            .collect()
    }

    fn hex_nibble(value: u8) -> u8 {
        match value {
            b'0'..=b'9' => value - b'0',
            b'a'..=b'f' => value - b'a' + 10,
            b'A'..=b'F' => value - b'A' + 10,
            _ => panic!("hex fixture contains a non-hex digit"),
        }
    }
}
