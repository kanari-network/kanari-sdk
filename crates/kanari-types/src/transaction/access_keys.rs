// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Shared "preferred access" resolution for transaction admission.
//!
//! Several mempool keys (primary lane, congestion lane, owner fallback) use the
//! same precedence: mutable object, first object, first gas-payment object,
//! then the sender. Keeping the precedence here prevents those keys from
//! drifting apart while still letting each caller format the source differently.

use super::{ObjectInput, ObjectRef, Transaction};
use move_core_types::account_address::AccountAddress;

/// Canonical sender form used by owner-namespace conflict keys.
pub(super) fn normalized_sender(sender: &str) -> String {
    if let Ok(addr) = AccountAddress::from_hex_literal(sender) {
        return addr.to_hex_literal();
    }
    if sender.starts_with("0x") { sender.to_string() } else { format!("0x{sender}") }
}

pub(super) enum PreferredAccess<'a> {
    MutObject(&'a ObjectInput),
    Object(&'a ObjectInput),
    GasPayment(&'a ObjectRef),
    Owner(&'a str),
}

pub(super) fn preferred_access(tx: &Transaction) -> PreferredAccess<'_> {
    let inputs = tx.object_inputs_ref();
    if let Some(input) = inputs.iter().find(|input| input.mutable) {
        return PreferredAccess::MutObject(input);
    }
    if let Some(input) = inputs.first() {
        return PreferredAccess::Object(input);
    }
    if let Some(payment) =
        tx.gas_payment_ref().and_then(|gas_payment| gas_payment.payment_objects.first())
    {
        return PreferredAccess::GasPayment(payment);
    }
    PreferredAccess::Owner(tx.sender())
}
