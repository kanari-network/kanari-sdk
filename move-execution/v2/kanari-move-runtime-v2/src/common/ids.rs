// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Shared identifier utilities for object and module IDs.
//!
//! The canonical object-id spelling is defined exactly once in the SDK —
//! [`kanari_types::object::canonical_object_id`] (full 64-hex lowercase).
//! This module only delegates to it. There are no alternate spellings, no
//! lookup variants, and no raw fallbacks: an id that does not canonicalize
//! never names an object.

use kanari_types::object as object_ids;

/// Canonical object id string for an address (full 64-hex).
pub(crate) use kanari_types::object::object_id_for_address;

/// Canonicalize an object ID to the full 64-hex form.
/// Returns `None` if the ID is not a valid hex-encoded Move address.
pub(crate) fn canonical_object_id(id: &str) -> Option<String> {
    object_ids::canonical_object_id(id)
}

/// Canonicalize an object id, asserting the input is a valid address.
///
/// Only for call sites where the id is valid by construction (storage keys,
/// derived ids, RPC-validated payloads). Invalid input is a programmer error.
pub(crate) fn canonical_object_id_invariant(id: &str) -> String {
    kanari_types::error::KanariUnwrapExt::invariant(
        object_ids::canonical_object_id(id),
        "object id must be a valid hex address",
    )
}

/// Canonical object id from raw bytes, or `None` for short or all-zero
/// sequences, which never name an object.
pub(crate) fn object_id_from_bytes(bytes: &[u8]) -> Option<String> {
    object_ids::object_id_from_bytes(bytes)
}
