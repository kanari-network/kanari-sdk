// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

/// Base64 and Base64URL encoding/decoding utilities
module kanari_system::base64 {

    /// Decodes a base64 or base64url encoded string into bytes
    /// Supports both standard base64 and base64url (URL-safe) encoding
    public fun decode(input: &vector<u8>): vector<u8> {
        native_decode(input)
    }

    native fun native_decode(input: &vector<u8>): vector<u8>;

    /// Encodes bytes into base64 string
    public fun encode(input: &vector<u8>): vector<u8> {
        native_encode(input)
    }

    native fun native_encode(input: &vector<u8>): vector<u8>;
}
