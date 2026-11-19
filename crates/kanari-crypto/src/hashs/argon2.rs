// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use argon2::PasswordVerifier;
/// Argon2 password hashing (memory-hard, quantum-resistant).
use argon2::{Argon2, PasswordHasher, password_hash::phc::PasswordHash};

/// Hash a password with Argon2id (default params).
#[must_use]
pub fn hash_password(password: &[u8]) -> String {
    let argon2 = Argon2::default();
    argon2.hash_password(password).map(|h| h.to_string()).unwrap_or_default()
}

/// Verify a password against an Argon2 hash.
pub fn verify_password(password: &[u8], hash: &str) -> bool {
    let parsed = match PasswordHash::new(hash) {
        Ok(h) => h,
        Err(_) => return false,
    };
    Argon2::default().verify_password(password, &parsed).is_ok()
}
