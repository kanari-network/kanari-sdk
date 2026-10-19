// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Native functions for `kanari_system::types`: type-convention checks.
//!
//! The one-time-witness check is a *convention* check only (struct name
//! equals the uppercased defining-module name, e.g. `my_coin::MY_COIN`).
//! It does not consult on-chain state: combine it with module-local
//! construction (only the defining module can pack the witness value) to
//! bound mint authority, as `coin` and `balance` do.

use better_any::{Tid, TidAble};
use move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes};
use move_core_types::language_storage::TypeTag;
use move_core_types::vm_status::StatusCode;
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::{NativeContext, NativeFunction};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::{NativeResult, PartialVMError, PartialVMResult};
use move_vm_types::pop_arg;
use move_vm_types::values::Value;
use move_vm_types::values::values_impl::Reference;
use smallvec::smallvec;
use std::collections::{BTreeSet, VecDeque};
use std::sync::Arc;

use crate::helpers::{expect_native_signature, make_module_natives};

/// Abort code when a witness type was already claimed.
pub const E_WITNESS_ALREADY_CLAIMED: u64 = 1;

/// Durable key holding the claimed-witness set (`Vec<String>`, sorted).
pub const CLAIMED_WITNESSES_KEY: &[u8] = b"claimed_witnesses";

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub is_one_time_witness: InternalGas,
    pub claim_witness: InternalGas,
    /// Per-byte rate over the rendered type name.
    pub per_byte: InternalGasPerByte,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            is_one_time_witness: 0.into(),
            claim_witness: 0.into(),
            per_byte: 0.into(),
        }
    }

    pub fn production() -> Self {
        Self {
            is_one_time_witness: 2_000.into(),
            claim_witness: 4_000.into(),
            per_byte: 50.into(),
        }
    }
}

/// Read-only view of the persisted witness-claim registry, provided by
/// the host.
pub trait WitnessClaimResolver: Send + Sync {
    /// All already-claimed witness type strings.
    fn claimed_witnesses(&self) -> Result<Vec<String>, String>;
}

/// Transaction-local record of witness claims. Mutations accumulate here
/// and are flushed to durable state by the host after a successful
/// transaction; the host persists them alongside everything else it
/// commits for the transaction.
#[derive(Tid, Clone, Default)]
pub struct ClaimedWitnessesExt {
    resolver: Option<Arc<dyn WitnessClaimResolver>>,
    pending: BTreeSet<String>,
}

impl ClaimedWitnessesExt {
    pub fn new(resolver: Arc<dyn WitnessClaimResolver>) -> Self {
        Self {
            resolver: Some(resolver),
            pending: BTreeSet::new(),
        }
    }

    /// Claim `type_str`, failing when already claimed in durable state or
    /// earlier in this transaction. Fail-closed and deterministic.
    pub fn claim(&mut self, type_str: &str) -> Result<(), String> {
        self.claim_key(type_str)
    }

    /// Claim `type_str` for a specific `purpose`, e.g. `"display"` or
    /// `"transfer_policy"`.
    ///
    /// Namespaced rather than sharing one flat set, because a single type may
    /// legitimately need several authorities — a coin type claims itself for
    /// currency creation while an NFT type claims itself for a Display and
    /// again for a TransferPolicy. A flat set would let one claim block the
    /// others.
    ///
    /// The stored key is `"{purpose}::{type}"`. Entries written before this
    /// existed are bare type strings (the currency case), and those stay valid:
    /// `claim()` keeps using the unprefixed form, so no state migration is
    /// needed and no existing claim is silently invalidated.
    pub fn claim_scoped(&mut self, purpose: &str, type_str: &str) -> Result<(), String> {
        self.claim_key(&format!("{purpose}::{type_str}"))
    }

    fn claim_key(&mut self, key: &str) -> Result<(), String> {
        if self.pending.contains(key) {
            return Err(format!("witness type already claimed: {key}"));
        }
        if let Some(resolver) = &self.resolver
            && resolver
                .claimed_witnesses()?
                .iter()
                .any(|claimed| claimed == key)
        {
            return Err(format!("witness type already claimed: {key}"));
        }
        self.pending.insert(key.to_string());
        Ok(())
    }

    /// Drain accumulated claims for the host to persist.
    pub fn take_pending(&mut self) -> BTreeSet<String> {
        std::mem::take(&mut self.pending)
    }
}

/// Sui-compatible convention: `T` is a one-time witness iff it is a struct
/// type named after its defining module in all caps.
fn is_otw_tag(tag: &TypeTag) -> bool {
    match tag {
        TypeTag::Struct(inner) => inner.module.as_str().to_ascii_uppercase() == inner.name.as_str(),
        _ => false,
    }
}

/// `types::is_one_time_witness<T: drop>(witness: &T): bool`.
fn native_is_one_time_witness(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(arguments.len(), 1, ty_args.len(), 1)?;

    // Borrowed witness; only its type matters.
    let _witness = pop_arg!(arguments, Reference);

    let tag = context.type_to_type_tag(&ty_args[0])?;
    let type_len = format!("{tag}").len() as u64;

    native_charge_gas_early_exit!(context, gas_params.is_one_time_witness);
    native_charge_gas_early_exit!(context, gas_params.per_byte * NumBytes::new(type_len));

    Ok(NR::ok(
        context.gas_used(),
        smallvec![Value::bool(is_otw_tag(&tag))],
    ))
}

pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let check_gas = gas_params.clone();
    let claim_gas = gas_params.clone();
    let display_gas = gas_params.clone();
    let policy_gas = gas_params;

    let is_one_time_witness: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_is_one_time_witness(&check_gas, context, ty_args, args)
    });
    let claim_witness: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_claim_witness(&claim_gas, context, ty_args, args)
    });
    let claim_display: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_claim_purpose(&display_gas, context, ty_args, args, PURPOSE_DISPLAY)
    });
    let claim_transfer_policy: NativeFunction = Arc::new(move |context, ty_args, args| {
        native_claim_purpose(&policy_gas, context, ty_args, args, PURPOSE_TRANSFER_POLICY)
    });

    make_module_natives([
        ("is_one_time_witness", is_one_time_witness),
        ("claim_witness", claim_witness),
        ("claim_display", claim_display),
        ("claim_transfer_policy", claim_transfer_policy),
    ])
}

/// Registry namespace for the one-Display-per-type claim.
const PURPOSE_DISPLAY: &str = "display";

/// Registry namespace for the one-policy-per-type claim.
const PURPOSE_TRANSFER_POLICY: &str = "transfer_policy";

/// Shared body for `claim_display` / `claim_transfer_policy`.
///
/// Claims `T` under `purpose` so exactly one authority of that kind can ever
/// exist for the type. This is what closes the unbound-`Publisher` gap:
/// consumers take `&Publisher<W>` for any `W` in the caller's package, so on
/// its own that proves only that the caller owns *some* type. Binding the claim
/// to `T` and refusing a second one makes the first authority created for a
/// type the only one that can exist — the guarantee Sui gets from its
/// host-side registry.
fn native_claim_purpose(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    arguments: VecDeque<Value>,
    purpose: &str,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(arguments.len(), 0, ty_args.len(), 1)?;

    let tag = context.type_to_type_tag(&ty_args[0])?;
    let type_str = format!("{tag}");

    native_charge_gas_early_exit!(context, gas_params.claim_witness);
    native_charge_gas_early_exit!(
        context,
        gas_params.per_byte * NumBytes::new(type_str.len() as u64)
    );

    let mut outcome: Option<Result<(), String>> = None;
    crate::native_ext::with_ext_mut_or_default::<ClaimedWitnessesExt, _>(context, |ext| {
        outcome = Some(ext.claim_scoped(purpose, &type_str));
    });

    match outcome {
        Some(Ok(())) => Ok(NR::ok(context.gas_used(), smallvec![])),
        Some(Err(error)) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_sub_status(E_WITNESS_ALREADY_CLAIMED)
            .with_message(error)),
        // No host registry (bare Move unit runner): allow. Runtime executions
        // always inject the extension, so this never weakens consensus.
        None => Ok(NR::ok(context.gas_used(), smallvec![])),
    }
}

/// `types::claim_witness<T: drop>(witness: &T)`.
///
/// Records `T` as claimed for currency/supply creation. Aborts
/// `E_WITNESS_ALREADY_CLAIMED` when `T` was already claimed in durable
/// state or earlier in this transaction, giving each coin type a single
/// canonical currency.
fn native_claim_witness(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    expect_native_signature(arguments.len(), 1, ty_args.len(), 1)?;

    // Borrowed witness; only its type matters.
    let _witness = pop_arg!(arguments, Reference);

    let tag = context.type_to_type_tag(&ty_args[0])?;
    let type_str = format!("{tag}");

    native_charge_gas_early_exit!(context, gas_params.claim_witness);
    native_charge_gas_early_exit!(
        context,
        gas_params.per_byte * NumBytes::new(type_str.len() as u64)
    );

    let mut outcome: Option<Result<(), String>> = None;
    crate::native_ext::with_ext_mut_or_default::<ClaimedWitnessesExt, _>(context, |ext| {
        outcome = Some(ext.claim(&type_str));
    });

    match outcome {
        Some(Ok(())) => Ok(NR::ok(context.gas_used(), smallvec![])),
        // Fail-closed: an unreadable registry must not mint a duplicate.
        Some(Err(error)) => Err(PartialVMError::new(StatusCode::ABORTED)
            .with_sub_status(E_WITNESS_ALREADY_CLAIMED)
            .with_message(error)),
        // No host registry (e.g. pure Move unit runner): allow. Runtime
        // executions always inject the extension, so this path never
        // weakens consensus enforcement.
        None => Ok(NR::ok(context.gas_used(), smallvec![])),
    }
}
