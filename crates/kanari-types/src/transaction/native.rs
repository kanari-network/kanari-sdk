// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum NativeCall {
    Transfer {
        coin_object_id: String,
        recipient: String,
        amount: u64,
    },
    Burn {
        amount: u64,
    },
}

impl NativeCall {
    pub fn tx_type_label(&self) -> &'static str {
        match self {
            Self::Transfer { .. } => "transfer",
            Self::Burn { .. } => "burn",
        }
    }

    pub fn required_native_amount(&self) -> u64 {
        match self {
            Self::Transfer { amount, .. } | Self::Burn { amount } => *amount,
        }
    }
}
