use serde::de::DeserializeOwned;

use crate::SecurityLevel;

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

#[derive(Debug)]
pub(crate) struct DecodedVerificationKey<Setup, Layouts> {
    pub setup: Setup,
    pub layouts: Layouts,
    pub security: SecurityLevel,
}

pub(crate) fn decode_verification_key<Setup: DeserializeOwned, Layouts: DeserializeOwned>(
    bytes: &[u8],
) -> Result<DecodedVerificationKey<Setup, Layouts>, String> {
    if !bytes.starts_with(&VERIFICATION_KEY_MAGIC) {
        return Err("verification key magic does not match expected value".to_string());
    }

    let encoded =
        crate::decode_exact::<EncodedVerificationKey<Setup, Layouts>>(bytes, "verification key")?;
    if encoded.magic != VERIFICATION_KEY_MAGIC {
        return Err("verification key magic does not match expected value".to_string());
    }
    if encoded.version != VERIFICATION_KEY_FORMAT_VERSION {
        return Err(format!(
            "unsupported verification key version {}",
            encoded.version
        ));
    }

    let security = SecurityLevel::from_wire_value(encoded.security)?;
    Ok(DecodedVerificationKey {
        setup: encoded.setup,
        layouts: encoded.layouts,
        security,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    const SECURITY_100_UNIT_KEY_HEX: &str =
        include_str!("../../../test_fixtures/verification_key_format/security_100_unit_key.hex");

    #[test]
    fn security_100_golden_key_decodes_security_tag() {
        let bytes = decode_hex_fixture(SECURITY_100_UNIT_KEY_HEX);

        let decoded =
            decode_verification_key::<(), ()>(&bytes).expect("decode golden unit verification key");

        assert!(matches!(decoded.security, SecurityLevel::Security100));
    }

    #[test]
    fn legacy_split_setup_does_not_decode_as_unified_key() {
        let legacy_setup = bincode::serde::encode_to_vec((), bincode::config::standard())
            .expect("encode legacy setup-like payload");

        let err = decode_verification_key::<(), ()>(&legacy_setup)
            .expect_err("reject non-envelope verification key");

        assert!(err.contains("magic"));
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
