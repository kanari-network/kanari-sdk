// Small hash abstraction layer using BLAKE3 (fast software hash).

/// Compute a 32-byte digest of `data` using BLAKE3.
pub fn digest(data: &[u8]) -> [u8; 32] {
    let h = blake3::hash(data);
    let mut out = [0u8; 32];
    out.copy_from_slice(h.as_bytes());
    out
}

/// Hash a leaf: H(0x00 || key_hash || value)
pub fn hash_leaf(key_hash: &[u8; 32], value: &[u8]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(1 + 32 + value.len());
    buf.push(0x00);
    buf.extend_from_slice(key_hash);
    buf.extend_from_slice(value);
    digest(&buf)
}

/// Hash an internal node: H(0x01 || left || right)
pub fn hash_node(left: &[u8; 32], right: &[u8; 32]) -> [u8; 32] {
    let mut buf = Vec::with_capacity(1 + 32 + 32);
    buf.push(0x01);
    buf.extend_from_slice(left);
    buf.extend_from_slice(right);
    digest(&buf)
}
