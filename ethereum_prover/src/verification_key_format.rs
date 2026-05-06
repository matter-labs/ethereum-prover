use crate::types::ProofSecurity;

pub(crate) const VERIFICATION_KEY_MAGIC: [u8; 8] = *b"EVKEY001";
const VERIFICATION_KEY_FORMAT_VERSION: u8 = 1;

#[derive(Debug, serde::Serialize, serde::Deserialize)]
struct EncodedVerificationKey<Setup, Layouts> {
    magic: [u8; 8],
    version: u8,
    security: u8,
    setup: Setup,
    layouts: Layouts,
}

pub(crate) fn encode_verification_key<Setup: serde::Serialize, Layouts: serde::Serialize>(
    setup: Setup,
    layouts: Layouts,
    security: ProofSecurity,
) -> Result<Vec<u8>, bincode::error::EncodeError> {
    // The canonical VK payload keeps Airbender's setup/layout split as internal
    // fields, but callers handle one opaque artifact. The magic/version prefix
    // lets verifiers reject legacy split files and future incompatible formats
    // with a clear error instead of attempting to deserialize them as setup data.
    let encoded = EncodedVerificationKey {
        magic: VERIFICATION_KEY_MAGIC,
        version: VERIFICATION_KEY_FORMAT_VERSION,
        security: security.proof_wire_value(),
        setup,
        layouts,
    };
    bincode::serde::encode_to_vec(&encoded, bincode::config::standard())
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECURITY_100_UNIT_KEY_HEX: &str =
        include_str!("../../test_fixtures/verification_key_format/security_100_unit_key.hex");

    #[test]
    fn encoded_key_starts_with_magic() {
        let encoded = EncodedVerificationKey {
            magic: VERIFICATION_KEY_MAGIC,
            version: VERIFICATION_KEY_FORMAT_VERSION,
            security: ProofSecurity::Security100.proof_wire_value(),
            setup: (),
            layouts: (),
        };

        let bytes = bincode::serde::encode_to_vec(&encoded, bincode::config::standard())
            .expect("encode test verification key");

        assert!(bytes.starts_with(&VERIFICATION_KEY_MAGIC));
    }

    #[test]
    fn security_100_unit_key_matches_golden_vector() {
        let bytes = encode_verification_key((), (), ProofSecurity::Security100)
            .expect("encode unit verification key");

        assert_eq!(to_hex(&bytes), SECURITY_100_UNIT_KEY_HEX.trim());
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
