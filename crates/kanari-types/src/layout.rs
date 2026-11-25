// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Byte layout primitives shared by SDK callers and runtime state codecs.
//!
//! These offsets are part of the persisted object format. They must stay
//! exact: no truncation, zero-padding, or alternate layout is acceptable.

/// Size of a Move `UID` (an `AccountAddress`) in bytes.
pub const MOVE_UID_BYTES: usize = 32;
/// Size of a serialized Move `u64` in bytes.
pub const MOVE_U64_BYTES: usize = 8;

/// Read a little-endian `u64` at an exact offset.
#[must_use]
pub fn read_u64_at(data: &[u8], offset: usize) -> Option<u64> {
    let end = offset.checked_add(MOVE_U64_BYTES)?;
    let bytes = data.get(offset..end)?;
    Some(u64::from_le_bytes(bytes.try_into().ok()?))
}

/// Write a little-endian `u64` at an exact offset.
pub fn write_u64_at(data: &mut [u8], offset: usize, value: u64) -> bool {
    let Some(end) = offset.checked_add(MOVE_U64_BYTES) else {
        return false;
    };
    let Some(bytes) = data.get_mut(offset..end) else {
        return false;
    };
    bytes.copy_from_slice(&value.to_le_bytes());
    true
}

/// Read a little-endian `u64` from the tail of a blob.
///
/// This mirrors `Balance<T>` write behavior when a blob carries trailing bytes
/// beyond the value.
#[must_use]
pub fn read_tail_u64(data: &[u8]) -> Option<u64> {
    if data.len() < MOVE_U64_BYTES {
        return None;
    }
    read_u64_at(data, data.len() - MOVE_U64_BYTES)
}

/// Write a little-endian `u64` to the tail of a blob.
pub fn write_tail_u64(data: &mut [u8], value: u64) -> bool {
    if data.len() < MOVE_U64_BYTES {
        return false;
    }
    let offset = data.len() - MOVE_U64_BYTES;
    write_u64_at(data, offset, value)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_offsets_do_not_accept_short_blobs() {
        assert_eq!(read_u64_at(&[0; 7], 0), None);
        assert_eq!(read_u64_at(&[0; 40], 32), Some(0));
        assert!(!write_u64_at(&mut [0; 39], 32, 1));
        assert!(write_u64_at(&mut [0; 40], 32, 1));
    }

    #[test]
    fn tail_helpers_preserve_little_endian_layout() {
        let mut data = vec![0xAA; 12];
        assert!(write_tail_u64(&mut data, 0x0102_0304_0506_0708));
        assert_eq!(&data[..4], &[0xAA; 4]);
        assert_eq!(read_tail_u64(&data), Some(0x0102_0304_0506_0708));
        assert_eq!(read_tail_u64(&[0; 7]), None);
    }
}
