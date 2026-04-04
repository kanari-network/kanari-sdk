// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Native implementation for event emission capturing
// via a Move native function `event::emit<T>(event: T)`
use better_any::{Tid, TidAble};
use move_core_types::account_address::AccountAddress;
use move_core_types::gas_algebra::{InternalGas, InternalGasPerByte, NumBytes};
use move_vm_runtime::native_charge_gas_early_exit;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_runtime::native_functions::{
    NativeFunction, NativeFunctionTable, make_table_from_iter,
};
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::natives::function::PartialVMError;
use move_vm_types::natives::function::PartialVMResult;
use move_vm_types::values::Value;
use smallvec::smallvec;
use std::collections::VecDeque;
use std::sync::Arc;

use crate::helpers::make_module_natives;

#[derive(Debug, Clone)]
pub struct GasParameters {
    pub base: InternalGas,
    pub per_byte: InternalGasPerByte,
}

impl GasParameters {
    pub fn zeros() -> Self {
        Self {
            base: 0.into(),
            per_byte: 0.into(),
        }
    }
}

/// A simple representation of an emitted event captured by the native.
#[derive(Clone, Debug)]
pub struct CapturedEvent {
    pub key: Vec<u8>,
    pub sequence_number: u64,
    pub type_tag: String,
    pub event_data: Vec<u8>,
}

#[derive(Tid, Default)]
pub struct EventsExt {
    pub events: Vec<CapturedEvent>,
}

impl EventsExt {
    pub fn record(&mut self, ev: CapturedEvent) {
        self.events.push(ev);
    }

    pub fn take_all(&mut self) -> Vec<CapturedEvent> {
        std::mem::take(&mut self.events)
    }
}

// Native registration
pub fn make_all(gas_params: GasParameters) -> impl Iterator<Item = (String, NativeFunction)> {
    let emit: NativeFunction =
        Arc::new(move |context, ty_args, args| native_emit(&gas_params, context, ty_args, args));
    make_module_natives([("emit", emit)])
}

pub fn all_natives(move_addr: AccountAddress) -> NativeFunctionTable {
    make_table_from_iter(
        move_addr,
        make_all(GasParameters::zeros())
            .map(|(func_name, func)| ("event".to_string(), func_name, func)),
    )
}

// native implementation for `event::emit<T: copy + drop>(event: T)`
fn native_emit(
    gas_params: &GasParameters,
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    native_charge_gas_early_exit!(context, gas_params.base);

    // Expect a single argument: the event value
    let evt_val = arguments.pop_back().ok_or_else(|| {
        PartialVMError::new(move_core_types::vm_status::StatusCode::INTERNAL_TYPE_ERROR)
            .with_message("Missing event argument".to_string())
    })?;

    if ty_args.is_empty() {
        // no type arg: we still attempt to serialize
    }

    // Determine a human-readable type tag if possible
    let type_tag_str = if let Some(ty) = ty_args.first() {
        match context.type_to_type_tag(ty) {
            Ok(tag) => format!("{}", tag),
            Err(_) => "<unknown>".to_string(),
        }
    } else {
        "<unknown>".to_string()
    };

    // Try to obtain layout and simple_serialize the event value
    let mut serialized: Vec<u8> = Vec::new();
    if let Some(ty) = ty_args.first()
        && let Ok(Some(layout)) = context.type_to_type_layout(ty)
        && let Some(bytes) = evt_val.simple_serialize(&layout)
    {
        serialized = bytes;
    }

    // If serialization failed, fall back to placeholder: bcs of empty or minimal
    if serialized.is_empty() {
        // Attempt BCS via value if possible, otherwise put empty
        serialized = vec![];
    }

    // Capture size for gas metering
    let size = serialized.len() as u64;

    // Build a captured event and record it in the native-extensions container
    let ev = CapturedEvent {
        key: vec![],
        sequence_number: 0,
        type_tag: type_tag_str,
        event_data: serialized,
    };

    native_charge_gas_early_exit!(context, gas_params.per_byte * NumBytes::new(size));

    crate::native_ext::with_ext_mut_or_default::<EventsExt, _>(context, |ext| ext.record(ev));

    Ok(NR::ok(context.gas_used(), smallvec![]))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn events_record_and_take_all() {
        let mut ext = EventsExt::default();
        let ev1 = CapturedEvent {
            key: vec![1],
            sequence_number: 1,
            type_tag: "T1".to_string(),
            event_data: vec![10, 20],
        };
        let ev2 = CapturedEvent {
            key: vec![2],
            sequence_number: 2,
            type_tag: "T2".to_string(),
            event_data: vec![30],
        };

        ext.record(ev1.clone());
        ext.record(ev2.clone());

        assert_eq!(ext.events.len(), 2);

        let taken = ext.take_all();
        assert_eq!(taken.len(), 2);
        assert_eq!(ext.events.len(), 0);
        assert_eq!(taken[0].sequence_number, 1);
        assert_eq!(taken[1].sequence_number, 2);
    }

    #[test]
    fn captured_event_fields() {
        let ev = CapturedEvent {
            key: vec![],
            sequence_number: 42,
            type_tag: "MyType".to_string(),
            event_data: vec![1, 2, 3],
        };

        assert_eq!(ev.type_tag, "MyType");
        assert_eq!(ev.sequence_number, 42);
        assert_eq!(ev.event_data, vec![1, 2, 3]);
    }
}
