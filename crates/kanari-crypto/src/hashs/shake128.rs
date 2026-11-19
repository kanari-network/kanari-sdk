// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Cryptographic hash using SHAKE128 with 256-bit output (quantum-resistant XOF).
#[must_use]
pub fn hash_data_shake128(data: &[u8]) -> Vec<u8> {
    hash_data_shake128_chunks(&[data])
}

/// Cryptographic hash using SHAKE128 over multiple input chunks with 256-bit output.
#[must_use]
pub fn hash_data_shake128_chunks(chunks: &[&[u8]]) -> Vec<u8> {
    hash_data_shake128_custom_chunks(chunks, 32)
}

/// Cryptographic hash using SHAKE128 with custom output length.
#[must_use]
pub fn hash_data_shake128_custom(data: &[u8], output_len: usize) -> Vec<u8> {
    hash_data_shake128_custom_chunks(&[data], output_len)
}

/// Cryptographic hash using SHAKE128 over multiple input chunks with custom output length.
#[must_use]
pub fn hash_data_shake128_custom_chunks(chunks: &[&[u8]], output_len: usize) -> Vec<u8> {
    use shake::{
        Shake128,
        digest::{ExtendableOutput, XofReader},
    };
    const MAX_SHAKE_OUTPUT: usize = 1024 * 1024; // 1MiB cap to prevent OOM
    let output_len = std::cmp::min(output_len, MAX_SHAKE_OUTPUT);
    let mut hasher = Shake128::default();
    for chunk in chunks {
        shake::Update::update(&mut hasher, chunk);
    }
    let mut reader = hasher.finalize_xof();
    let mut output = vec![0u8; output_len];
    reader.read(&mut output);
    output
}
