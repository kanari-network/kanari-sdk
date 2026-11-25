// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Small identity/encoding rules shared by node configuration and RPC.

/// Canonical `0x`-prefixed form of a consensus authority ID.
///
/// This is intentionally not object-ID canonicalization: authority IDs are
/// configuration identities, so casing and padding must be preserved.
#[must_use]
pub fn normalize_0x_prefixed_id(value: impl AsRef<str>) -> String {
    let value = value.as_ref();
    if value.starts_with("0x") { value.to_owned() } else { format!("0x{value}") }
}

/// Decode hex with an optional `0x` prefix and require an exact byte length.
pub fn decode_hex_exact(label: &str, value: &str, expected_len: usize) -> anyhow::Result<Vec<u8>> {
    let value = value.strip_prefix("0x").unwrap_or(value);
    let bytes =
        hex::decode(value).map_err(|error| anyhow::anyhow!("Invalid {label} hex: {error}"))?;
    if bytes.len() != expected_len {
        anyhow::bail!("Invalid {label} length: expected {expected_len} bytes, got {}", bytes.len());
    }
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn authority_ids_only_gain_a_prefix() {
        assert_eq!(normalize_0x_prefixed_id("1"), "0x1");
        assert_eq!(normalize_0x_prefixed_id("0xAbC"), "0xAbC");
    }

    #[test]
    fn hex_decode_requires_exact_length() {
        assert_eq!(decode_hex_exact("test key", "0x0102", 2).unwrap(), [1, 2]);
        assert!(decode_hex_exact("test key", "01", 2).is_err());
        assert!(decode_hex_exact("test key", "0xzz", 1).is_err());
    }
}
