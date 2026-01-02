// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Native implementation for event emission capturing
// via a Move native function `event::emit<T>(event: T)`
use crate::make_native;
use better_any::{Tid, TidAble};
use move_core_types::account_address::AccountAddress;
use move_core_types::gas_algebra::GasQuantity;
use move_vm_runtime::native_functions::NativeContext;
use move_vm_runtime::native_functions::make_table_from_iter;
use move_vm_types::loaded_data::runtime_types::Type;
use move_vm_types::natives::function::NativeResult;
use move_vm_types::natives::function::PartialVMResult;
use move_vm_types::values::Value;
use smallvec::smallvec;
use std::collections::VecDeque;

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
pub fn all_natives(
    move_addr: AccountAddress,
) -> move_vm_runtime::native_functions::NativeFunctionTable {
    let natives = vec![("event", "emit", make_native(native_emit))];
    make_table_from_iter(move_addr, natives)
}

// native implementation for `event::emit<T: copy + drop>(event: T)`
fn native_emit(
    context: &mut NativeContext,
    ty_args: Vec<Type>,
    mut arguments: VecDeque<Value>,
) -> PartialVMResult<NativeResult> {
    use move_vm_types::natives::function::NativeResult as NR;

    let gas_used_now = context.gas_used();

    // Expect a single argument: the event value
    let evt_val = arguments.pop_back().expect("Missing event argument");

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

    // Build a captured event and record it in the native-extensions container
    let exts = context.extensions_mut();

    let ev = CapturedEvent {
        key: vec![],
        sequence_number: 0,
        type_tag: type_tag_str,
        event_data: serialized,
    };

    // Attempt to record into the runtime native-extensions container. In some hosts
    // (for example the Move unit-test runner) the `EventsExt` may not be registered
    // on the session. `get_mut::<EventsExt>()` will panic in that case, which would
    // abort the whole test runner. To be robust, catch panics and skip recording
    // when the extension is absent.
    let _ = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        exts.get_mut::<EventsExt>().record(ev);
    }));

    // Small gas cost for event emission
    let gas_cost = GasQuantity::new(500);

    Ok(NR::ok(gas_used_now + gas_cost, smallvec![]))
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
