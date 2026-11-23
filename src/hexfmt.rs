//! Lower-case hex formatting for digests.
//!
//! RustCrypto 0.11 digests (`sha2` 0.11) return `hybrid_array::Array`, which no
//! longer implements `LowerHex`, so `format!("{:x}", digest)` stopped compiling.

use std::fmt::Write as _;

/// Format bytes as a lower-case hexadecimal string.
pub fn hex_lower(bytes: impl AsRef<[u8]>) -> String {
    let bytes = bytes.as_ref();
    let mut out = String::with_capacity(bytes.len() * 2);
    for b in bytes {
        let _ = write!(out, "{b:02x}");
    }
    out
}

#[cfg(test)]
mod tests {
    use super::hex_lower;

    #[test]
    fn formats_bytes() {
        assert_eq!(hex_lower([0x00, 0x0f, 0xab, 0xff]), "000fabff");
        assert_eq!(hex_lower([] as [u8; 0]), "");
    }
}
