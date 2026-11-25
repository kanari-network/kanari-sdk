// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Cryptographic hash using Blake3 (faster alternative).
#[must_use]
pub fn hash_data_blake3(data: &[u8]) -> Vec<u8> {
    hash_data_blake3_array(data).to_vec()
}

/// Cryptographic hash using Blake3 and returning a fixed 32-byte array.
#[must_use]
pub fn hash_data_blake3_array(data: &[u8]) -> [u8; 32] {
    hash_small(data)
}

/// Cryptographic hash using Blake3 over multiple input chunks.
///
/// This keeps callers from materializing temporary concatenation buffers when
/// hashing domain-separated structured data such as SMT leaves and nodes.
#[must_use]
pub fn hash_data_blake3_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    hash_data_blake3_chunks_array(chunks).to_vec()
}

/// Cryptographic hash using Blake3 over multiple input chunks, returning a fixed 32-byte array.
#[must_use]
pub fn hash_data_blake3_chunks_array(chunks: &[&[u8]]) -> [u8; 32] {
    // SMT leaf/node hashing is dominated by a handful of tiny, fixed chunks.
    // Concatenating them into a stack buffer lets BLAKE3 process one contiguous
    // input instead of performing several incremental updates. This is exactly
    // the same byte stream as the generic path, just cheaper per call.
    const SMALL_BUFFER_LEN: usize = 256;
    let total_len = chunks.iter().fold(0usize, |total, chunk| total + chunk.len());
    if total_len <= SMALL_BUFFER_LEN {
        let mut buffer = [0u8; SMALL_BUFFER_LEN];
        let mut offset = 0usize;
        for chunk in chunks {
            buffer[offset..offset + chunk.len()].copy_from_slice(chunk);
            offset += chunk.len();
        }
        return hash_small(&buffer[..offset]);
    }

    let mut hasher = blake3::Hasher::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    *hasher.finalize().as_bytes()
}

#[inline]
fn hash_small(data: &[u8]) -> [u8; 32] {
    let mut hasher = blake3::Hasher::new();
    hasher.update(data);
    *hasher.finalize().as_bytes()
}

/// BLAKE3 keyed hash (MAC) with a 32-byte key.
#[must_use]
pub fn hash_data_blake3_keyed(key: &[u8; 32], data: &[u8]) -> Vec<u8> {
    hash_data_blake3_keyed_chunks(key, &[data])
}

#[must_use]
pub fn hash_data_blake3_keyed_chunks(key: &[u8; 32], chunks: &[&[u8]]) -> Vec<u8> {
    let mut hasher = blake3::Hasher::new_keyed(key);
    for chunk in chunks {
        hasher.update(chunk);
    }
    hasher.finalize().as_bytes().to_vec()
}

/// BLAKE3 XOF (extendable output) with custom length.
#[must_use]
pub fn hash_data_blake3_xof(data: &[u8], output_len: usize) -> Vec<u8> {
    hash_data_blake3_xof_chunks(&[data], output_len)
}

#[must_use]
pub fn hash_data_blake3_xof_chunks(chunks: &[&[u8]], output_len: usize) -> Vec<u8> {
    const MAX_OUTPUT: usize = 1024 * 1024;
    let output_len = std::cmp::min(output_len, MAX_OUTPUT);
    let mut hasher = blake3::Hasher::new();
    for chunk in chunks {
        hasher.update(chunk);
    }
    let mut reader = hasher.finalize_xof();
    let mut output = vec![0u8; output_len];
    reader.fill(&mut output);
    output
}

/// BLAKE3 key derivation (KDF).
#[must_use]
pub fn hash_data_blake3_derive_key(context: &str, key_material: &[u8]) -> [u8; 32] {
    blake3::derive_key(context, key_material)
}
