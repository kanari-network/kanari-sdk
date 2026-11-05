// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Custom argument deserialization.

use serde::Deserializer;
use serde::de::{self, SeqAccess, Visitor};
use std::fmt;

/// Custom deserializer for args that supports both hex strings and byte arrays
pub fn deserialize_args<'de, D>(deserializer: D) -> Result<Vec<Vec<u8>>, D::Error>
where
    D: Deserializer<'de>,
{
    struct ArgsVisitor;

    impl<'de> Visitor<'de> for ArgsVisitor {
        type Value = Vec<Vec<u8>>;

        fn expecting(&self, formatter: &mut fmt::Formatter) -> fmt::Result {
            formatter.write_str("array of hex strings or byte arrays")
        }

        fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
        where
            A: SeqAccess<'de>,
        {
            let mut args = Vec::new();
            while let Some(value) = seq.next_element::<serde_json::Value>()? {
                // Try to parse as hex string first
                if let Some(hex_str) = value.as_str() {
                    let hex_str = hex_str.trim_start_matches("0x");
                    let bytes = hex::decode(hex_str).map_err(de::Error::custom)?;
                    args.push(bytes);
                } else if let Some(arr) = value.as_array() {
                    // Parse as array of numbers
                    let bytes: Result<Vec<u8>, _> = arr
                        .iter()
                        .map(|v| {
                            v.as_u64()
                                .ok_or_else(|| de::Error::custom("Expected number"))
                        })
                        .map(|r| r.map(|n| n as u8))
                        .collect();
                    args.push(bytes?);
                } else {
                    return Err(de::Error::custom("Expected string or array"));
                }
            }
            Ok(args)
        }
    }

    deserializer.deserialize_seq(ArgsVisitor)
}
