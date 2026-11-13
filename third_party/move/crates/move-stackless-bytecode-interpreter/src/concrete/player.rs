// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

//! This file implements the statement interpretation part of the stackless bytecode interpreter.

use std::{collections::BTreeMap, rc::Rc};

use num::{BigInt, ToPrimitive, Zero};

use bytecode_interpreter_crypto::{
    blake2b256_of, blake3_256_of, ed25519_deserialize_public_key, ed25519_deserialize_signature,
    ed25519_verify_signature, keccak256_of, ripemd160_of, sha2_256_of, sha3_256_of,
};
use move_binary_format::errors::Location;
use move_core_types::{
    account_address::AccountAddress,
    language_storage::CORE_CODE_ADDRESS,
    u256,
    vm_status::{StatusCode, sub_status},
};
use move_model::{
    ast::{Attribute, TempIndex},
    model::{FunId, FunctionEnv, ModuleId, StructId},
    ty as MT,
};
use move_stackless_bytecode::{
    function_target::FunctionTarget,
    function_target_pipeline::FunctionTargetsHolder,
    stackless_bytecode::{
        AbortAction, AssignKind, BorrowEdge, BorrowNode, Bytecode, Constant, HavocKind, Label,
        Operation,
    },
};

use crate::{
    concrete::{
        local_state::{AbortInfo, LocalState, TerminationStatus},
        settings::InterpreterSettings,
        ty::{
            BaseType, CodeOffset, StructField, StructInstantiation, Type, convert_model_base_type,
            convert_model_local_type, convert_model_struct_type,
        },
        value::{BaseValue, EvalState, GlobalState, LocalSlot, Pointer, TypedValue},
    },
    shared::{
        ident::{ModuleIdent, StructIdent},
        variant::choose_variant,
    },
};

//**************************************************************************************************
// Types
//**************************************************************************************************

pub type ExecResult<T> = ::std::result::Result<T, AbortInfo>;

//**************************************************************************************************
// Constants
//**************************************************************************************************

// TODO(mengxu): these constants are defined in values_impl.rs which are currently not exposed.
const INDEX_OUT_OF_BOUNDS: u64 = sub_status::NFE_VECTOR_ERROR_BASE + 1;
const POP_EMPTY_VEC: u64 = sub_status::NFE_VECTOR_ERROR_BASE + 2;
const DESTROY_NON_EMPTY_VEC: u64 = sub_status::NFE_VECTOR_ERROR_BASE + 3;
const VECTOR_TOO_LONG: u64 = sub_status::NFE_VECTOR_ERROR_BASE + 4;

/// Maximum value depth for type layout construction, mirroring
/// `move_vm_runtime::VALUE_DEPTH_MAX` (now publicly re-exported).
const VALUE_DEPTH_MAX: u64 = move_vm_runtime::VALUE_DEPTH_MAX;

/// Maximum vector length enforced by the Move VM, mirroring
/// `move_vm_config::verifier::DEFAULT_MAX_CONSTANT_VECTOR_LEN`.
const VECTOR_LEN_MAX: u64 = move_vm_config::verifier::DEFAULT_MAX_CONSTANT_VECTOR_LEN;

/// Compute the depth of a base type: the number of type nodes along the longest path from
/// the root to a leaf, matching how the Move VM counts depth when building type layouts.
fn base_type_depth(ty: &BaseType) -> u64 {
    match ty {
        BaseType::Primitive(_) => 1,
        BaseType::Vector(elem) => 1 + base_type_depth(elem),
        BaseType::Struct(inst) => {
            1 + inst
                .fields
                .iter()
                .map(|field| base_type_depth(&field.ty))
                .max()
                .unwrap_or(0)
        }
    }
}

/// Convert a (non-negative) big integer into a `u256` value. Returns `None` if the value is
/// negative or does not fit into 256 bits.
fn big_int_to_u256(v: &BigInt) -> Option<u256::U256> {
    use num::bigint::Sign;
    if v.sign() == Sign::Minus || v.bits() > 256 {
        return None;
    }
    let bytes = v.to_bytes_le().1;
    let mut buf = [0u8; u256::U256_NUM_BYTES];
    buf[..bytes.len()].copy_from_slice(&bytes);
    Some(u256::U256::from_le_bytes(&buf))
}

//**************************************************************************************************
// Execution context
//**************************************************************************************************

pub struct FunctionContext<'env> {
    // context
    holder: &'env FunctionTargetsHolder,
    target: FunctionTarget<'env>,
    ty_args: Vec<BaseType>,
    label_offsets: BTreeMap<Label, CodeOffset>,
    // debug
    level: usize,
}

impl<'env> FunctionContext<'env> {
    pub fn new(
        holder: &'env FunctionTargetsHolder,
        target: FunctionTarget<'env>,
        ty_args: Vec<BaseType>,
        level: usize,
    ) -> Self {
        let label_offsets = Bytecode::label_offsets(target.get_bytecode());
        Self {
            holder,
            target,
            ty_args,
            label_offsets,
            level,
        }
    }

    //
    // settings
    //

    /// Retrieve the `InterpreterSettings` from the global environment
    pub fn get_settings(&self) -> Rc<InterpreterSettings> {
        self.target
            .global_env()
            .get_extension::<InterpreterSettings>()
            .unwrap_or_default()
    }

    //
    // execution
    //

    /// Collect addresses stored in the value recursively
    fn collect_addresses(val: &BaseValue, addresses: &mut Vec<AccountAddress>) {
        match val {
            BaseValue::Address(v) | BaseValue::Signer(v) => {
                addresses.push(*v);
            }
            BaseValue::Vector(vec) | BaseValue::Struct(vec) => {
                for val in vec.iter() {
                    Self::collect_addresses(val, addresses);
                }
            }
            BaseValue::Bool(_) => (),
            BaseValue::Int(_) => (),
        }
    }

    /// Execute a user function with value arguments.
    pub fn exec_user_function(
        &self,
        skip_specs: bool,
        typed_args: Vec<TypedValue>,
        global_state: &mut GlobalState,
        eval_state: &mut EvalState,
    ) -> ExecResult<LocalState> {
        // collect addresses
        for typed_arg in typed_args.iter() {
            let mut addresses = vec![];
            Self::collect_addresses(typed_arg.get_val(), &mut addresses);
            global_state.put_touched_addresses(&addresses);
        }

        // execute the user function
        let instructions = self.target.get_bytecode();
        let debug_bytecode = self.get_settings().verbose_bytecode;
        let mut local_state = self.prepare_local_state(skip_specs, typed_args);
        while !local_state.is_terminated() {
            let pc = local_state.get_pc() as usize;
            let bytecode = instructions.get(pc).unwrap();
            if debug_bytecode {
                println!(
                    "{} {}[{}]: {}",
                    "-".repeat(self.level),
                    self.target.func_env.get_full_name_str(),
                    pc,
                    bytecode.display(&self.target, &self.label_offsets)
                );
            }
            self.exec_bytecode(bytecode, &mut local_state, global_state, eval_state)?;
        }
        Ok(local_state)
    }

    /// Execute a native function with the type arguments and value arguments.
    fn exec_native_function(
        &self,
        srcs: &[TempIndex],
        typed_args: Vec<TypedValue>,
        local_state: &mut LocalState,
        global_state: &mut GlobalState,
    ) -> ExecResult<Vec<TypedValue>> {
        let mut dummy_state = self.prepare_local_state(local_state.is_spec_skipped(), typed_args);
        if cfg!(debug_assertions) {
            assert_eq!(dummy_state.num_slots(), srcs.len());
        }

        // locate
        let env = self.target.global_env();
        let addr = *self.target.module_env().self_address();
        let module_name = env
            .symbol_pool()
            .string(self.target.module_env().get_name().name());
        let function_name = env.symbol_pool().string(self.target.get_name());

        // dispatch
        match (addr, module_name.as_str(), function_name.as_str()) {
            (CORE_CODE_ADDRESS, "vector", "empty") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 0);
                }
                let res = self.native_vector_empty();
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "vector", "length") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_vector_length(dummy_state.del_value(0));
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "vector", "borrow") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 2);
                }
                self.native_vector_borrow(dummy_state.del_value(0), dummy_state.del_value(1))
                    .map(|res| vec![res])
            }
            (CORE_CODE_ADDRESS, "vector", "borrow_mut") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 2);
                }
                self.native_vector_borrow_mut(
                    *srcs.first().unwrap(),
                    dummy_state.del_value(0),
                    dummy_state.del_value(1),
                )
                .map(|res| vec![res])
            }
            (CORE_CODE_ADDRESS, "vector", "push_back") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 2);
                }
                match self
                    .native_vector_push_back(dummy_state.del_value(0), dummy_state.del_value(1))
                {
                    Ok(res) => {
                        local_state.put_value_override(*srcs.first().unwrap(), res);
                        Ok(vec![])
                    }
                    Err(e) => Err(e),
                }
            }
            (CORE_CODE_ADDRESS, "vector", "pop_back") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_vector_pop_back(dummy_state.del_value(0));
                match res {
                    Ok((new_vec, elem_val)) => {
                        local_state.put_value_override(*srcs.first().unwrap(), new_vec);
                        Ok(vec![elem_val])
                    }
                    Err(e) => Err(e),
                }
            }
            (CORE_CODE_ADDRESS, "vector", "destroy_empty") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_vector_destroy_empty(dummy_state.del_value(0));
                match res {
                    Ok(_) => {
                        local_state.del_value(*srcs.first().unwrap());
                        Ok(vec![])
                    }
                    Err(e) => Err(e),
                }
            }
            (CORE_CODE_ADDRESS, "vector", "swap") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 3);
                }
                let res = self.native_vector_swap(
                    dummy_state.del_value(0),
                    dummy_state.del_value(1),
                    dummy_state.del_value(2),
                );
                match res {
                    Ok(new_vec) => {
                        local_state.put_value_override(*srcs.first().unwrap(), new_vec);
                        Ok(vec![])
                    }
                    Err(e) => Err(e),
                }
            }
            (CORE_CODE_ADDRESS, "signer", "borrow_address") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_signer_borrow_address(dummy_state.del_value(0));
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "hash", "sha2_256") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_hash_sha2_256(dummy_state.del_value(0));
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "hash", "sha3_256") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_hash_sha3_256(dummy_state.del_value(0));
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "hash", "keccak256") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_hash_keccak256(dummy_state.del_value(0));
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "hash", "blake2b256") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_hash_blake2b256(dummy_state.del_value(0));
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "hash", "blake3_256") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_hash_blake3_256(dummy_state.del_value(0));
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "hash", "ripemd160") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_hash_ripemd160(dummy_state.del_value(0));
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "signer", "address_to_u64") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_signer_address_to_u64(dummy_state.del_value(0));
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "signer", "address_to_bytes") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_signer_address_to_bytes(dummy_state.del_value(0));
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "string", "internal_check_utf8") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_string_internal_check_utf8(dummy_state.del_value(0));
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "string", "internal_is_char_boundary") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 2);
                }
                let bytes_val = dummy_state.del_value(0);
                let idx_val = dummy_state.del_value(1);
                let res = self.native_string_internal_is_char_boundary(bytes_val, idx_val);
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "string", "internal_sub_string") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 3);
                }
                let bytes_val = dummy_state.del_value(0);
                let i_val = dummy_state.del_value(1);
                let j_val = dummy_state.del_value(2);
                self.native_string_internal_sub_string(bytes_val, i_val, j_val)
                    .map(|res| vec![res])
            }
            (CORE_CODE_ADDRESS, "string", "internal_index_of") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 2);
                }
                let s_val = dummy_state.del_value(0);
                let r_val = dummy_state.del_value(1);
                let res = self.native_string_internal_index_of(s_val, r_val);
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "type_name", "get")
            | (CORE_CODE_ADDRESS, "type_name", "get_with_original_ids") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 0);
                }
                let res = self.native_type_name_get();
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "bcs", "to_bytes") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                self.native_bcs_to_bytes(dummy_state.del_value(0))
                    .map(|res| vec![res])
            }
            (CORE_CODE_ADDRESS, "event", "write_to_event_store") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 3);
                }
                self.native_event_write_to_event_store(
                    dummy_state.del_value(0),
                    dummy_state.del_value(1),
                    dummy_state.del_value(2),
                    global_state,
                );
                Ok(vec![])
            }
            (CORE_CODE_ADDRESS, "Signature", "ed25519_validate_pubkey") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                let res = self.native_signature_ed25519_validate_pubkey(dummy_state.del_value(0));
                Ok(vec![res])
            }
            (CORE_CODE_ADDRESS, "Signature", "ed25519_verify") => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 3);
                }
                let res = self.native_signature_ed25519_signature_verification(
                    dummy_state.del_value(0),
                    dummy_state.del_value(1),
                    dummy_state.del_value(2),
                );
                Ok(vec![res])
            }
            _ => unreachable!(),
        }
    }

    fn exec_bytecode(
        &self,
        bytecode: &Bytecode,
        local_state: &mut LocalState,
        global_state: &mut GlobalState,
        eval_state: &mut EvalState,
    ) -> ExecResult<()> {
        match bytecode {
            Bytecode::Assign(_, dst, src, kind) => {
                self.handle_assign(*dst, *src, kind, local_state)
            }
            Bytecode::Load(_, dst, constant) => self.handle_load(*dst, constant, local_state),
            Bytecode::Call(_, dsts, op, srcs, on_abort) => self.handle_operation(
                dsts,
                op,
                srcs,
                on_abort.as_ref(),
                local_state,
                global_state,
                eval_state,
            )?,
            Bytecode::Label(_, label) => {
                if cfg!(debug_assertions) {
                    self.code_offset_by_label(*label);
                }
            }
            Bytecode::Jump(_, label) => {
                local_state.set_pc(self.code_offset_by_label(*label));
            }
            Bytecode::Branch(_, then_label, else_label, cond) => {
                self.handle_conditional_branch(*cond, *then_label, *else_label, local_state)
            }
            Bytecode::Abort(_, index) => self.handle_abort(*index, local_state),
            Bytecode::Ret(_, rets) => self.handle_return(rets, local_state),
            Bytecode::Nop(_) => (),
        }
        local_state.ready_pc_for_next_instruction();
        Ok(())
    }

    //
    // per-bytecode processing
    //

    fn handle_assign(
        &self,
        dst: TempIndex,
        src: TempIndex,
        kind: &AssignKind,
        local_state: &mut LocalState,
    ) {
        let into_val = match kind {
            AssignKind::Move => {
                let from_val = local_state.del_value(src);
                from_val.assign_cast(local_state.get_type(dst).clone())
            }
            AssignKind::Copy => {
                let from_val = local_state.get_value(src);
                from_val.assign_cast(local_state.get_type(dst).clone())
            }
            AssignKind::Store => {
                let from_val = local_state.get_value(src);
                let into_ty = local_state.get_type(dst).clone();
                if from_val.get_ty().is_ref(Some(true)) {
                    from_val.borrow_direct(into_ty, src)
                } else {
                    from_val.assign_cast(into_ty)
                }
            }
        };
        local_state.put_value_override(dst, into_val);
    }

    fn handle_load(&self, dst: TempIndex, constant: &Constant, local_state: &mut LocalState) {
        let val = match constant {
            Constant::Bool(v) => TypedValue::mk_bool(*v),
            Constant::U8(v) => TypedValue::mk_u8(*v),
            Constant::U16(v) => TypedValue::mk_u16(*v),
            Constant::U32(v) => TypedValue::mk_u32(*v),
            Constant::U64(v) => TypedValue::mk_u64(*v),
            Constant::U128(v) => TypedValue::mk_u128(*v),
            Constant::U256(v) => TypedValue::mk_u256(u256::U256::from_le_bytes(&v.to_le_bytes())),
            Constant::Address(v) => TypedValue::mk_address(
                AccountAddress::from_hex_literal(&format!("{:#x}", v)).unwrap(),
            ),
            Constant::ByteArray(v) => {
                let elems = v.iter().map(|e| TypedValue::mk_u8(*e)).collect();
                TypedValue::mk_vector(BaseType::mk_u8(), elems)
            }
            Constant::AddressArray(v) => {
                let elems = v
                    .iter()
                    .map(|e| {
                        TypedValue::mk_address(
                            AccountAddress::from_hex_literal(&format!("{:#x}", *e)).unwrap(),
                        )
                    })
                    .collect();
                TypedValue::mk_vector(BaseType::mk_address(), elems)
            }
            Constant::Vector(v) => {
                let elem_ty = match local_state.get_type(dst).get_base_type() {
                    BaseType::Vector(elem_ty) => (**elem_ty).clone(),
                    _ => unreachable!("Load of a vector constant into a non-vector local"),
                };
                let elems = v
                    .iter()
                    .map(|e| self.constant_to_typed_value(e, &elem_ty))
                    .collect();
                TypedValue::mk_vector(elem_ty, elems)
            }
        };
        local_state.put_value_override(dst, val);
    }

    /// Convert a constant into a typed value of the given base type.
    /// The base type must be compatible with the constant.
    fn constant_to_typed_value(&self, constant: &Constant, ty: &BaseType) -> TypedValue {
        if cfg!(debug_assertions) {
            match constant {
                Constant::Bool(_) => assert!(ty.is_bool()),
                Constant::U8(_) => assert!(ty.is_u8()),
                Constant::U16(_) => assert!(ty.is_u16()),
                Constant::U32(_) => assert!(ty.is_u32()),
                Constant::U64(_) => assert!(ty.is_u64()),
                Constant::U128(_) => assert!(ty.is_u128()),
                Constant::U256(_) => assert!(ty.is_u256()),
                Constant::Address(_) => assert!(ty.is_address()),
                Constant::ByteArray(_) => assert!(ty.is_vector_of(&BaseType::mk_u8())),
                Constant::AddressArray(_) => assert!(ty.is_vector_of(&BaseType::mk_address())),
                Constant::Vector(_) => assert!(ty.is_vector()),
            }
        }
        match constant {
            Constant::Bool(v) => TypedValue::mk_bool(*v),
            Constant::U8(v) => TypedValue::mk_u8(*v),
            Constant::U16(v) => TypedValue::mk_u16(*v),
            Constant::U32(v) => TypedValue::mk_u32(*v),
            Constant::U64(v) => TypedValue::mk_u64(*v),
            Constant::U128(v) => TypedValue::mk_u128(*v),
            Constant::U256(v) => TypedValue::mk_u256(u256::U256::from_le_bytes(&v.to_le_bytes())),
            Constant::Address(v) => TypedValue::mk_address(
                AccountAddress::from_hex_literal(&format!("{:#x}", v)).unwrap(),
            ),
            Constant::ByteArray(v) => {
                let elems = v.iter().map(|e| TypedValue::mk_u8(*e)).collect();
                TypedValue::mk_vector(BaseType::mk_u8(), elems)
            }
            Constant::AddressArray(v) => {
                let elems = v
                    .iter()
                    .map(|e| {
                        TypedValue::mk_address(
                            AccountAddress::from_hex_literal(&format!("{:#x}", *e)).unwrap(),
                        )
                    })
                    .collect();
                TypedValue::mk_vector(BaseType::mk_address(), elems)
            }
            Constant::Vector(v) => {
                let elem_ty = match ty {
                    BaseType::Vector(elem_ty) => (**elem_ty).clone(),
                    _ => unreachable!("Vector constant with a non-vector element type"),
                };
                let elems = v
                    .iter()
                    .map(|e| self.constant_to_typed_value(e, &elem_ty))
                    .collect();
                TypedValue::mk_vector(elem_ty, elems)
            }
        }
    }

    fn handle_operation(
        &self,
        dsts: &[TempIndex],
        op: &Operation,
        srcs: &[TempIndex],
        on_abort: Option<&AbortAction>,
        local_state: &mut LocalState,
        global_state: &mut GlobalState,
        eval_state: &mut EvalState,
    ) -> ExecResult<()> {
        // check abort handler
        if cfg!(debug_assertions) {
            match on_abort {
                None => (),
                Some(action) => {
                    assert!(op.can_abort());
                    assert!(
                        local_state
                            .get_type(action.1)
                            .is_compatible_for_abort_code()
                    );
                }
            }
        }

        // operations that does not need to have the argument in storage
        match op {
            // built-ins
            Operation::Uninit => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                self.handle_uninit(srcs[0], local_state);
                return Ok(());
            }
            Operation::Destroy => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                }
                self.handle_destroy(srcs[0], local_state);
                return Ok(());
            }
            Operation::Stop => {
                // we should never see the Stop operation in interpreter mode
                unreachable!()
            }
            Operation::Havoc(kind) => {
                if cfg!(debug_assertions) {
                    assert_eq!(dsts.len(), 1);
                    let target_ty = local_state.get_type(*dsts.first().unwrap());
                    match kind {
                        HavocKind::Value => {
                            assert!(target_ty.is_base());
                        }
                        HavocKind::MutationValue | HavocKind::MutationAll => {
                            assert!(target_ty.is_ref(Some(true)));
                        }
                    }
                }
                return Ok(());
            }
            // debugging
            Operation::TraceLocal(index) => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                    assert_eq!(local_state.get_type(*index), local_state.get_type(srcs[0]));
                }
                return Ok(());
            }
            Operation::TraceReturn(num) => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                    assert!(*num < self.target.get_return_count());
                }
                return Ok(());
            }
            Operation::TraceAbort => {
                if cfg!(debug_assertions) {
                    assert_eq!(srcs.len(), 1);
                    assert!(local_state.get_type(srcs[0]).is_compatible_for_abort_code());
                }
                return Ok(());
            }
            // all others require args to be collected up front
            _ => (),
        }

        // collect arguments
        let mut typed_args: Vec<_> = srcs.iter().map(|idx| local_state.get_value(*idx)).collect();

        // case on operation type
        let op_result = match op {
            // function call
            Operation::Function(module_id, fun_id, ty_args) => self.handle_call_function(
                *module_id,
                *fun_id,
                ty_args,
                typed_args,
                srcs,
                local_state,
                global_state,
                eval_state,
            ),
            // opaque
            Operation::OpaqueCallBegin(module_id, fun_id, ty_args) => self.handle_call_function(
                *module_id,
                *fun_id,
                ty_args,
                typed_args,
                srcs,
                local_state,
                global_state,
                eval_state,
            ),
            Operation::OpaqueCallEnd(module_id, fun_id, ty_args) => {
                self.handle_opaque_call_end(*module_id, *fun_id, ty_args, typed_args);
                Ok(vec![])
            }
            // struct
            Operation::Pack(module_id, struct_id, ty_args) => {
                let packed = self.handle_pack(*module_id, *struct_id, ty_args, typed_args);
                Ok(vec![packed])
            }
            Operation::Unpack(module_id, struct_id, ty_args) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                let unpacked =
                    self.handle_unpack(*module_id, *struct_id, ty_args, typed_args.remove(0));
                Ok(unpacked)
            }
            Operation::GetField(module_id, struct_id, ty_args, field_num) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                let field = self.handle_get_field(
                    *module_id,
                    *struct_id,
                    ty_args,
                    *field_num,
                    typed_args.remove(0),
                );
                Ok(vec![field])
            }
            Operation::BorrowField(module_id, struct_id, ty_args, field_num) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                let (is_mut, _) = local_state.get_type(dsts[0]).get_ref_type();
                let field = self.handle_borrow_field(
                    *module_id,
                    *struct_id,
                    ty_args,
                    *field_num,
                    is_mut,
                    srcs[0],
                    typed_args.remove(0),
                );
                Ok(vec![field])
            }
            Operation::MoveTo(module_id, struct_id, ty_args) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 2);
                }
                self.handle_move_to(
                    *module_id,
                    *struct_id,
                    ty_args,
                    typed_args.remove(1),
                    typed_args.remove(0),
                    global_state,
                )
                .map(|_| Vec::new())
            }
            Operation::MoveFrom(module_id, struct_id, ty_args) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                self.handle_move_from(
                    *module_id,
                    *struct_id,
                    ty_args,
                    typed_args.remove(0),
                    global_state,
                )
                .map(|object| vec![object])
            }
            Operation::GetGlobal(module_id, struct_id, ty_args) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                self.handle_get_global(
                    *module_id,
                    *struct_id,
                    ty_args,
                    typed_args.remove(0),
                    global_state,
                )
                .map(|object| vec![object])
            }
            Operation::BorrowGlobal(module_id, struct_id, ty_args) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                let (is_mut, _) = local_state.get_type(dsts[0]).get_ref_type();
                self.handle_borrow_global(
                    *module_id,
                    *struct_id,
                    ty_args,
                    is_mut,
                    typed_args.remove(0),
                    global_state,
                )
                .map(|object| vec![object])
            }
            Operation::Exists(module_id, struct_id, ty_args) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                let exists = self.handle_exists_global(
                    *module_id,
                    *struct_id,
                    ty_args,
                    typed_args.remove(0),
                    global_state,
                );
                Ok(vec![exists])
            }
            // scope
            Operation::PackRef
            | Operation::UnpackRef
            | Operation::PackRefDeep
            | Operation::UnpackRefDeep => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                    let arg_ty = typed_args.first().unwrap().get_ty();
                    assert!(arg_ty.is_struct() || arg_ty.is_ref_struct(Some(true)));
                }
                Ok(vec![])
            }
            // write-back
            Operation::IsParent(BorrowNode::Reference(parent_idx), edge) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                let result = self.handle_is_parent(*parent_idx, edge, typed_args.remove(0));
                Ok(vec![result])
            }
            Operation::IsParent(_, _) => {
                // only Reference can appear in BorrowNode
                unreachable!()
            }
            Operation::WriteBack(BorrowNode::GlobalRoot(qid), edge) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                match edge {
                    BorrowEdge::Direct => self.handle_write_back_global_struct(
                        qid.module_id,
                        qid.id,
                        &qid.inst,
                        typed_args.remove(0),
                        global_state,
                    ),
                    _ => unreachable!(),
                }
                Ok(vec![])
            }
            Operation::WriteBack(BorrowNode::LocalRoot(idx), edge) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                match edge {
                    BorrowEdge::Direct => {
                        self.handle_write_back_local(*idx, typed_args.remove(0), local_state)
                    }
                    _ => unreachable!(),
                }
                Ok(vec![])
            }
            Operation::WriteBack(BorrowNode::Reference(idx), edge) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                match edge {
                    BorrowEdge::Direct => {
                        self.handle_write_back_ref_direct(*idx, typed_args.remove(0), local_state)
                    }
                    BorrowEdge::Field(qid, field_num) => self.handle_write_back_ref_field(
                        qid.module_id,
                        qid.id,
                        &qid.inst,
                        *idx,
                        *field_num,
                        typed_args.remove(0),
                        local_state,
                    ),
                    BorrowEdge::Index(_) => {
                        self.handle_write_back_ref_element(*idx, typed_args.remove(0), local_state)
                    }
                    BorrowEdge::Hyper(hyper) => self.handle_write_back_ref_hyper(
                        hyper,
                        *idx,
                        typed_args.remove(0),
                        local_state,
                    ),
                }
                Ok(vec![])
            }
            Operation::WriteBack(BorrowNode::ReturnPlaceholder(_), ..) => {
                // this node should never appear in bytecode
                unreachable!()
            }
            // references
            Operation::BorrowLoc => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                let (is_mut, _) = local_state.get_type(dsts[0]).get_ref_type();
                let object = self.handle_borrow_local(is_mut, typed_args.remove(0), srcs[0]);
                Ok(vec![object])
            }
            Operation::ReadRef => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                let object = self.handle_read_ref(typed_args.remove(0));
                Ok(vec![object])
            }
            Operation::WriteRef => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 2);
                }
                self.handle_write_ref(typed_args.remove(1), srcs[0], local_state);
                Ok(vec![])
            }
            Operation::FreezeRef => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                let object = self.handle_freeze_ref(typed_args.remove(0));
                Ok(vec![object])
            }
            // cast
            Operation::CastU8
            | Operation::CastU16
            | Operation::CastU32
            | Operation::CastU64
            | Operation::CastU128
            | Operation::CastU256 => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                let val = typed_args.remove(0);
                match op {
                    Operation::CastU8 => self.handle_cast_u8(val),
                    Operation::CastU16 => self.handle_cast_u16(val),
                    Operation::CastU32 => self.handle_cast_u32(val),
                    Operation::CastU64 => self.handle_cast_u64(val),
                    Operation::CastU128 => self.handle_cast_u128(val),
                    Operation::CastU256 => self.handle_cast_u256(val),
                    _ => unreachable!(),
                }
                .map(|casted| vec![casted])
            }
            // binary arithmetic
            Operation::Add | Operation::Sub | Operation::Mul | Operation::Div | Operation::Mod => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 2);
                }
                let rhs = typed_args.remove(1);
                let lhs = typed_args.remove(0);
                self.handle_binary_arithmetic(op, lhs, rhs, local_state.get_type(dsts[0]))
                    .map(|calculated| vec![calculated])
            }
            // binary bitwise
            Operation::BitAnd | Operation::BitOr | Operation::Xor => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 2);
                }
                let rhs = typed_args.remove(1);
                let lhs = typed_args.remove(0);
                let calculated =
                    self.handle_binary_bitwise(op, lhs, rhs, local_state.get_type(dsts[0]));
                Ok(vec![calculated])
            }
            // binary bitshift
            Operation::Shl | Operation::Shr => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 2);
                }
                let rhs = typed_args.remove(1);
                let lhs = typed_args.remove(0);
                let calculated =
                    self.handle_binary_bitshift(op, lhs, rhs, local_state.get_type(dsts[0]));
                Ok(vec![calculated])
            }
            // binary comparison
            Operation::Lt | Operation::Le | Operation::Ge | Operation::Gt => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 2);
                }
                let rhs = typed_args.remove(1);
                let lhs = typed_args.remove(0);
                let calculated =
                    self.handle_binary_comparison(op, lhs, rhs, local_state.get_type(dsts[0]));
                Ok(vec![calculated])
            }
            // binary equality
            Operation::Eq | Operation::Neq => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 2);
                }
                let rhs = typed_args.remove(1);
                let lhs = typed_args.remove(0);
                let calculated =
                    self.handle_binary_equality(op, lhs, rhs, local_state.get_type(dsts[0]));
                Ok(vec![calculated])
            }
            // unary boolean
            Operation::Not => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 1);
                }
                let opv = typed_args.remove(0);
                let calculated = self.handle_unary_boolean(op, opv, local_state.get_type(dsts[0]));
                Ok(vec![calculated])
            }
            // binary boolean
            Operation::And | Operation::Or => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_args.len(), 2);
                }
                let rhs = typed_args.remove(1);
                let lhs = typed_args.remove(0);
                let calculated =
                    self.handle_binary_boolean(op, lhs, rhs, local_state.get_type(dsts[0]));
                Ok(vec![calculated])
            }
            // already handled
            Operation::Stop
            | Operation::Uninit
            | Operation::Destroy
            | Operation::Havoc(..)
            | Operation::TraceLocal(..)
            | Operation::TraceReturn(..)
            | Operation::TraceAbort => {
                unreachable!();
            }
        };

        // handle result
        match op_result {
            Ok(typed_rets) => {
                if cfg!(debug_assertions) {
                    assert_eq!(typed_rets.len(), dsts.len());
                }
                for (typed_ret, &idx) in typed_rets.into_iter().zip(dsts) {
                    local_state.put_value_override(idx, typed_ret);
                }
            }
            Err(abort_info) => match on_abort {
                None => {
                    return Err(abort_info);
                }
                Some(action) => {
                    let abort_idx = action.1;
                    let abort_val = if local_state.get_type(abort_idx).is_u64() {
                        TypedValue::mk_u64(abort_info.get_status_code())
                    } else {
                        TypedValue::mk_num(BigInt::from(abort_info.get_status_code()))
                    };
                    local_state.put_value(abort_idx, abort_val);
                    local_state.set_pc(self.code_offset_by_label(action.0));
                    local_state.transit_to_post_abort(abort_info);
                }
            },
        }
        Ok(())
    }

    fn handle_call_function(
        &self,
        module_id: ModuleId,
        fun_id: FunId,
        ty_args: &[MT::Type],
        typed_args: Vec<TypedValue>,
        srcs: &[TempIndex],
        local_state: &mut LocalState,
        global_state: &mut GlobalState,
        eval_state: &mut EvalState,
    ) -> ExecResult<Vec<TypedValue>> {
        let env = self.target.global_env();
        let callee_env = env.get_function(module_id.qualified(fun_id));
        let callee_ctxt = self.derive_callee_ctxt(&callee_env, ty_args);

        // check argument count and types
        if cfg!(debug_assertions) {
            assert_eq!(callee_ctxt.target.get_parameter_count(), typed_args.len());
        }

        // short-circuit the execution if this is a native function
        if callee_env.is_native() {
            return callee_ctxt
                .exec_native_function(srcs, typed_args, local_state, global_state)
                .map_err(|abort_info| self.remap_native_abort_info(&callee_env, abort_info));
        }

        // collect mutable arguments
        let mut_args: BTreeMap<_, _> = typed_args
            .iter()
            .enumerate()
            .filter(|(_, arg)| arg.get_ty().is_ref(Some(true)))
            .map(|(callee_idx, _)| (callee_idx, srcs[callee_idx]))
            .collect();

        // wrap the pointer in mut_ref args
        let typed_args = typed_args
            .into_iter()
            .enumerate()
            .map(|(idx, arg)| {
                if mut_args.contains_key(&idx) {
                    arg.box_into_mut_ref_arg(srcs[idx])
                } else {
                    arg
                }
            })
            .collect();

        // execute the user function
        let mut callee_state = callee_ctxt.exec_user_function(
            local_state.is_spec_skipped(),
            typed_args,
            global_state,
            eval_state,
        )?;

        // update mutable arguments
        for (callee_idx, origin_idx) in mut_args {
            let old_val = local_state.del_value(origin_idx);
            let new_val = if callee_state.has_value(callee_idx) {
                callee_state.del_value(callee_idx)
            } else {
                callee_state.load_destroyed_arg(callee_idx)
            }
            .unbox_from_mut_ref_arg();
            if cfg!(debug_assertions) {
                assert_eq!(old_val.get_ptr(), new_val.get_ptr());
            }
            local_state.put_value(origin_idx, new_val);
        }

        // check callee termination status
        let termination = callee_state.into_termination_status();
        match termination {
            TerminationStatus::Abort(abort_info) => Err(abort_info),
            TerminationStatus::Return(return_vals) => Ok(return_vals),
            TerminationStatus::None | TerminationStatus::PostAbort(_) => unreachable!(),
        }
    }

    /// Natives marked with `#[bytecode_instruction]` (e.g., the `std::vector` primitives) are
    /// compiled into bytecode instructions executed in the caller's code. To match what the
    /// Move VM reports, their aborts must use the vector operation error status and be
    /// attributed to the caller's module instead of the module declaring the native.
    fn remap_native_abort_info(
        &self,
        callee_env: &FunctionEnv<'env>,
        abort_info: AbortInfo,
    ) -> AbortInfo {
        let env = self.target.global_env();
        let is_instruction_native = callee_env.get_attributes().iter().any(|attr| {
            matches!(
                attr,
                Attribute::Apply(_, sym, _)
                    if env.symbol_pool().string(*sym).as_str() == "bytecode_instruction"
            )
        });
        if !is_instruction_native {
            // The Move VM attributes errors that occur inside a native call to the caller's
            // frame (e.g., `VM_MAX_VALUE_DEPTH_REACHED` from the BCS layout construction is
            // reported at the calling module). System aborts raised by the stackless
            // interpreter inside a native carry the native's module location; re-attribute
            // them to the caller's module to match the Move VM.
            return match abort_info {
                AbortInfo::Internal(status, _) => {
                    AbortInfo::Internal(status, self.module_location())
                }
                other => other,
            };
        }
        match abort_info {
            AbortInfo::User(status_code, _)
                if status_code == INDEX_OUT_OF_BOUNDS
                    || status_code == POP_EMPTY_VEC
                    || status_code == DESTROY_NON_EMPTY_VEC
                    || status_code == VECTOR_TOO_LONG =>
            {
                AbortInfo::InternalWithSubStatus(
                    StatusCode::VECTOR_OPERATION_ERROR,
                    status_code,
                    self.module_location(),
                )
            }
            other => other,
        }
    }

    fn handle_opaque_call_end(
        &self,
        module_id: ModuleId,
        fun_id: FunId,
        ty_args: &[MT::Type],
        typed_args: Vec<TypedValue>,
    ) {
        if cfg!(debug_assertions) {
            let env = self.target.global_env();
            let callee_env = env.get_function(module_id.qualified(fun_id));
            let callee_ctxt = self.derive_callee_ctxt(&callee_env, ty_args);
            assert_eq!(callee_ctxt.target.get_parameter_count(), typed_args.len());
        }
    }

    fn handle_pack(
        &self,
        module_id: ModuleId,
        struct_id: StructId,
        ty_args: &[MT::Type],
        op_fields: Vec<TypedValue>,
    ) -> TypedValue {
        let env = self.target.global_env();
        let inst = convert_model_struct_type(env, module_id, struct_id, ty_args, &self.ty_args);
        if cfg!(debug_assertions) {
            assert_eq!(inst.fields.len(), op_fields.len());
        }
        TypedValue::mk_struct(inst, op_fields)
    }

    fn handle_unpack(
        &self,
        module_id: ModuleId,
        struct_id: StructId,
        ty_args: &[MT::Type],
        op_struct: TypedValue,
    ) -> Vec<TypedValue> {
        if cfg!(debug_assertions) {
            let env = self.target.global_env();
            let inst = convert_model_struct_type(env, module_id, struct_id, ty_args, &self.ty_args);
            assert_eq!(&inst, op_struct.get_ty().get_struct_inst());
        }
        op_struct.unpack_struct()
    }

    fn handle_get_field(
        &self,
        module_id: ModuleId,
        struct_id: StructId,
        ty_args: &[MT::Type],
        field_num: usize,
        op_struct: TypedValue,
    ) -> TypedValue {
        if cfg!(debug_assertions) {
            let env = self.target.global_env();
            let inst = convert_model_struct_type(env, module_id, struct_id, ty_args, &self.ty_args);
            assert!(
                op_struct.get_ty().is_struct_of(&inst)
                    || op_struct.get_ty().is_ref_struct_of(&inst, None)
            );
        }
        if op_struct.get_ty().is_struct() {
            op_struct.unpack_struct_field(field_num)
        } else {
            op_struct.unpack_ref_struct_field(field_num, None)
        }
    }

    fn handle_borrow_field(
        &self,
        module_id: ModuleId,
        struct_id: StructId,
        ty_args: &[MT::Type],
        field_num: usize,
        is_mut: bool,
        local_idx: TempIndex,
        op_struct: TypedValue,
    ) -> TypedValue {
        if cfg!(debug_assertions) {
            let env = self.target.global_env();
            let inst = convert_model_struct_type(env, module_id, struct_id, ty_args, &self.ty_args);
            assert_eq!(&inst, op_struct.get_ty().get_ref_struct_inst(None));
        }
        op_struct.borrow_ref_struct_field(field_num, is_mut, local_idx)
    }

    fn handle_move_to(
        &self,
        module_id: ModuleId,
        struct_id: StructId,
        ty_args: &[MT::Type],
        op_signer: TypedValue,
        op_struct: TypedValue,
        global_state: &mut GlobalState,
    ) -> Result<(), AbortInfo> {
        let env = self.target.global_env();
        let inst = convert_model_struct_type(env, module_id, struct_id, ty_args, &self.ty_args);
        let addr = op_signer.into_signer();
        if global_state.has_resource(&addr, &inst) {
            return Err(self.sys_abort(StatusCode::RESOURCE_ALREADY_EXISTS));
        }
        global_state.put_resource(addr, inst, op_struct);
        Ok(())
    }

    fn handle_move_from(
        &self,
        module_id: ModuleId,
        struct_id: StructId,
        ty_args: &[MT::Type],
        op_addr: TypedValue,
        global_state: &mut GlobalState,
    ) -> Result<TypedValue, AbortInfo> {
        let env = self.target.global_env();
        let inst = convert_model_struct_type(env, module_id, struct_id, ty_args, &self.ty_args);
        let addr = op_addr.into_address();
        match global_state.del_resource(addr, inst) {
            None => Err(self.sys_abort(StatusCode::MISSING_DATA)),
            Some(object) => Ok(object),
        }
    }

    fn handle_get_global(
        &self,
        module_id: ModuleId,
        struct_id: StructId,
        ty_args: &[MT::Type],
        op_addr: TypedValue,
        global_state: &mut GlobalState,
    ) -> Result<TypedValue, AbortInfo> {
        let env = self.target.global_env();
        let inst = convert_model_struct_type(env, module_id, struct_id, ty_args, &self.ty_args);
        let addr = op_addr.into_address();
        match global_state.get_resource_for_code(None, addr, inst) {
            None => Err(self.sys_abort(StatusCode::MISSING_DATA)),
            Some(object) => Ok(object),
        }
    }

    fn handle_borrow_global(
        &self,
        module_id: ModuleId,
        struct_id: StructId,
        ty_args: &[MT::Type],
        is_mut: bool,
        op_addr: TypedValue,
        global_state: &mut GlobalState,
    ) -> Result<TypedValue, AbortInfo> {
        let env = self.target.global_env();
        let inst = convert_model_struct_type(env, module_id, struct_id, ty_args, &self.ty_args);
        let addr = op_addr.into_address();
        match global_state.get_resource_for_code(Some(is_mut), addr, inst) {
            None => Err(self.sys_abort(StatusCode::MISSING_DATA)),
            Some(object) => Ok(object),
        }
    }

    fn handle_exists_global(
        &self,
        module_id: ModuleId,
        struct_id: StructId,
        ty_args: &[MT::Type],
        op_addr: TypedValue,
        global_state: &GlobalState,
    ) -> TypedValue {
        let env = self.target.global_env();
        let inst = convert_model_struct_type(env, module_id, struct_id, ty_args, &self.ty_args);
        let addr = op_addr.into_address();
        TypedValue::mk_bool(global_state.has_resource(&addr, &inst))
    }

    fn handle_is_parent(
        &self,
        parent_idx: TempIndex,
        edge: &BorrowEdge,
        op_val: TypedValue,
    ) -> TypedValue {
        fn follow_pointer_edge(p: &Pointer, e: &BorrowEdge) -> bool {
            match (p, e) {
                (Pointer::RefField(_, p_field_num), BorrowEdge::Field(_, e_field_num)) => {
                    p_field_num == e_field_num
                }
                (Pointer::RefElement(_, _), BorrowEdge::Index(_)) => true,
                _ => false,
            }
        }

        fn follow_pointer_trace(trace: &[Pointer], edges: &[BorrowEdge]) -> bool {
            for (p, e) in trace.iter().rev().zip(edges.iter()) {
                if !follow_pointer_edge(p, e) {
                    return false;
                }
            }
            true
        }

        let (_, _, ptr) = op_val.decompose();
        let is_parent = match ptr {
            Pointer::RefDirect(idx) => idx == parent_idx,
            Pointer::RefField(idx, _) => idx == parent_idx,
            Pointer::RefElement(idx, _) => idx == parent_idx,
            Pointer::ArgRef(idx, _) => idx == parent_idx,
            Pointer::RetRef(mut trace) => match trace.pop().unwrap() {
                Pointer::ArgRef(idx, _) => {
                    if idx == parent_idx {
                        if trace.len() == 1 {
                            follow_pointer_edge(trace.first().unwrap(), edge)
                        } else {
                            match edge {
                                BorrowEdge::Hyper(hyper) if hyper.len() == trace.len() => {
                                    follow_pointer_trace(&trace, hyper)
                                }
                                _ => false,
                            }
                        }
                    } else {
                        false
                    }
                }
                _ => unreachable!(),
            },
            Pointer::None | Pointer::Local(_) | Pointer::Global(_) => unreachable!(),
        };
        TypedValue::mk_bool(is_parent)
    }

    fn handle_write_back_global_struct(
        &self,
        module_id: ModuleId,
        struct_id: StructId,
        ty_args: &[MT::Type],
        op_struct: TypedValue,
        global_state: &mut GlobalState,
    ) {
        let env = self.target.global_env();
        let inst = convert_model_struct_type(env, module_id, struct_id, ty_args, &self.ty_args);
        let addr = match op_struct.get_ptr() {
            // TODO (mengxu) this needs to be extended to check for actual address in borrow graph
            // only put the resource back when the address matches
            Pointer::Global(addr) => *addr,
            _ => unreachable!(),
        };
        let old_resource = global_state.put_resource(addr, inst, op_struct.read_ref());
        if cfg!(debug_assertions) {
            assert!(old_resource.is_some());
        }
    }

    fn handle_write_back_local(
        &self,
        local_root: TempIndex,
        op_val: TypedValue,
        local_state: &mut LocalState,
    ) {
        if cfg!(debug_assertions) {
            assert!(
                op_val
                    .get_ty()
                    .is_ref_of(local_state.get_type(local_root).get_base_type(), Some(true))
            );
            assert!(local_state.has_value(local_root));
        }
        match op_val.get_ptr() {
            Pointer::Local(root_idx) => {
                if cfg!(debug_assertions) {
                    assert_eq!(*root_idx, local_root);
                }
                local_state.put_value_override(local_root, op_val.read_ref());
            }
            _ => unreachable!(),
        }
    }

    fn handle_write_back_ref_direct(
        &self,
        local_ref: TempIndex,
        op_val: TypedValue,
        local_state: &mut LocalState,
    ) {
        let old_val = local_state.del_value(local_ref);
        if cfg!(debug_assertions) {
            let new_ty = op_val.get_ty();
            assert!(new_ty.is_ref(Some(true)));
            assert_eq!(new_ty, old_val.get_ty());
        }

        let new_val = match op_val.get_ptr() {
            Pointer::RefDirect(parent_idx) => {
                if cfg!(debug_assertions) {
                    assert_eq!(*parent_idx, local_ref);
                }
                old_val.update_ref_direct(op_val)
            }
            Pointer::RetRef(trace) => {
                // check pointer validity
                if cfg!(debug_assertions) {
                    assert_eq!(trace.len(), 1);
                    match trace.first().unwrap() {
                        Pointer::ArgRef(ref_idx, original_ptr) => {
                            assert_eq!(*ref_idx, local_ref);
                            assert_eq!(original_ptr.as_ref(), old_val.get_ptr());
                        }
                        _ => unreachable!(),
                    }
                }
                op_val.unbox_from_mut_ref_ret()
            }
            _ => unreachable!(),
        };
        local_state.put_value(local_ref, new_val);
    }

    fn handle_write_back_ref_field(
        &self,
        module_id: ModuleId,
        struct_id: StructId,
        ty_args: &[MT::Type],
        local_ref: TempIndex,
        field_num: usize,
        op_val: TypedValue,
        local_state: &mut LocalState,
    ) {
        let old_struct = local_state.del_value(local_ref);
        if cfg!(debug_assertions) {
            let env = self.target.global_env();
            let inst = convert_model_struct_type(env, module_id, struct_id, ty_args, &self.ty_args);
            assert!(old_struct.get_ty().is_ref_struct_of(&inst, Some(true)));

            // check pointer validity
            match op_val.get_ptr() {
                Pointer::RefField(ref_idx, ref_field) => {
                    assert_eq!(*ref_field, field_num);
                    assert_eq!(*ref_idx, local_ref);
                }
                Pointer::RetRef(trace) => {
                    assert_eq!(trace.len(), 2);
                    match trace.get(1).unwrap() {
                        Pointer::ArgRef(ref_idx, _) => {
                            assert_eq!(*ref_idx, local_ref);
                        }
                        _ => unreachable!(),
                    }
                    match trace.first().unwrap() {
                        Pointer::RefField(_, ref_field) => {
                            assert_eq!(*ref_field, field_num);
                        }
                        _ => unreachable!(),
                    }
                }
                _ => unreachable!(),
            };
        }
        let new_struct = old_struct.update_ref_struct_field(field_num, op_val);
        local_state.put_value(local_ref, new_struct);
    }

    fn handle_write_back_ref_element(
        &self,
        local_ref: TempIndex,
        op_val: TypedValue,
        local_state: &mut LocalState,
    ) {
        let old_vector = local_state.del_value(local_ref);
        let elem_num = match op_val.get_ptr() {
            Pointer::RefElement(ref_idx, elem_num) => {
                if cfg!(debug_assertions) {
                    assert_eq!(*ref_idx, local_ref);
                }
                elem_num
            }
            Pointer::RetRef(trace) => {
                assert_eq!(trace.len(), 2);
                match trace.get(1).unwrap() {
                    Pointer::ArgRef(ref_idx, _) => {
                        assert_eq!(*ref_idx, local_ref);
                    }
                    _ => unreachable!(),
                }
                match trace.first().unwrap() {
                    Pointer::RefElement(_, elem_num) => elem_num,
                    _ => unreachable!(),
                }
            }
            _ => unreachable!(),
        };
        let new_vector = old_vector.update_ref_vector_element(*elem_num, op_val);
        local_state.put_value(local_ref, new_vector);
    }

    fn handle_write_back_ref_hyper(
        &self,
        edges: &[BorrowEdge],
        local_ref: TempIndex,
        op_val: TypedValue,
        local_state: &mut LocalState,
    ) {
        let new_val = match op_val.get_ptr() {
            Pointer::RetRef(trace) => {
                let steps = edges.len();
                if cfg!(debug_assertions) {
                    assert_eq!(trace.len(), steps + 1);
                    match trace.last().unwrap() {
                        Pointer::ArgRef(ref_idx, _) => assert_eq!(*ref_idx, local_ref),
                        _ => unreachable!(),
                    }
                }

                let mut cur = local_state.del_value(local_ref);
                let mut path = vec![];
                for (i, edge) in edges.iter().enumerate() {
                    let ptr = trace.get(steps - 1 - i).unwrap();
                    let sub = match (ptr, edge) {
                        (
                            Pointer::RefField(callee_idx, p_field_num),
                            BorrowEdge::Field(qid, field_num),
                        ) => {
                            if cfg!(debug_assertions) {
                                let env = self.target.global_env();
                                let inst = convert_model_struct_type(
                                    env,
                                    qid.module_id,
                                    qid.id,
                                    &qid.inst,
                                    &self.ty_args,
                                );
                                assert!(cur.get_ty().is_ref_struct_of(&inst, Some(true)));
                                assert_eq!(p_field_num, field_num);
                            }
                            path.push(cur.clone());
                            // NOTE: the local_idx argument can be any dummy value here
                            cur.borrow_ref_struct_field(*field_num, true, *callee_idx)
                        }
                        (Pointer::RefElement(callee_idx, elem_num), BorrowEdge::Index(_)) => {
                            if cfg!(debug_assertions) {
                                assert!(cur.get_ty().is_ref_vector(Some(true)));
                            }
                            path.push(cur.clone());
                            // NOTE: the local_idx argument can be any dummy value here
                            cur.borrow_ref_vector_element(*elem_num, true, *callee_idx)
                                .unwrap()
                        }
                        _ => unreachable!(),
                    };
                    cur = sub;
                }

                if cfg!(debug_assertions) {
                    let new_ty = op_val.get_ty();
                    assert!(new_ty.is_ref(Some(true)));
                    assert_eq!(new_ty, cur.get_ty());
                }

                // TODO (mengxu): refactor the code to remove this clone
                let mut cur = op_val.clone();
                for (i, (val, edge)) in path.into_iter().zip(edges.iter()).rev().enumerate() {
                    let ptr = trace.get(i).unwrap();
                    let sub = match edge {
                        BorrowEdge::Field(_, field_num) => {
                            val.update_ref_struct_field(*field_num, cur)
                        }
                        BorrowEdge::Index(_) => {
                            let elem_num = match ptr {
                                Pointer::RefElement(_, elem_num) => elem_num,
                                _ => unreachable!(),
                            };
                            val.update_ref_vector_element(*elem_num, cur)
                        }
                        _ => unreachable!(),
                    };
                    cur = sub;
                }
                cur
            }
            _ => unreachable!(),
        };
        local_state.put_value(local_ref, new_val);
    }

    fn handle_borrow_local(
        &self,
        is_mut: bool,
        local_val: TypedValue,
        local_idx: TempIndex,
    ) -> TypedValue {
        local_val.borrow_local(is_mut, local_idx)
    }

    fn handle_read_ref(&self, from_ref: TypedValue) -> TypedValue {
        from_ref.read_ref()
    }

    fn handle_write_ref(
        &self,
        from_val: TypedValue,
        into_ref: TempIndex,
        local_state: &mut LocalState,
    ) {
        let old_val = local_state.del_value(into_ref);
        let (old_ty, _, old_ptr) = old_val.decompose();
        if cfg!(debug_assertions) {
            assert!(old_ty.is_ref_of(from_val.get_ty().get_base_type(), Some(true)));
        }
        let new_val = from_val.write_ref(old_ptr);
        local_state.put_value(into_ref, new_val);
    }

    fn handle_freeze_ref(&self, ref_val: TypedValue) -> TypedValue {
        ref_val.freeze_ref()
    }

    fn handle_uninit(&self, local_idx: TempIndex, local_state: &mut LocalState) {
        if cfg!(debug_assertions) {
            assert!(!local_state.has_value(local_idx));
        }
        local_state.mark_uninit(local_idx);
    }

    fn handle_destroy(&self, local_idx: TempIndex, local_state: &mut LocalState) {
        if local_state.unset_uninit(local_idx) {
            if cfg!(debug_assertions) {
                assert!(!local_state.has_value(local_idx));
            }
        } else {
            let val = local_state.del_value(local_idx);
            if local_idx < self.target.get_parameter_count() {
                local_state.save_destroyed_arg(local_idx, val);
            }
        }
    }

    fn handle_cast_u8(&self, val: TypedValue) -> Result<TypedValue, AbortInfo> {
        let (ty, val, _) = val.decompose();
        let v = if ty.is_u8() {
            val.into_u8()
        } else if ty.is_u16() {
            let v = val.into_u16();
            if v > (u8::MAX as u16) {
                return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
            }
            v as u8
        } else if ty.is_u32() {
            let v = val.into_u32();
            if v > (u8::MAX as u32) {
                return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
            }
            v as u8
        } else if ty.is_u64() {
            let v = val.into_u64();
            if v > (u8::MAX as u64) {
                return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
            }
            v as u8
        } else if ty.is_u128() {
            let v = val.into_u128();
            if v > (u8::MAX as u128) {
                return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
            }
            v as u8
        } else {
            let n = val.into_num();
            match n.to_u8() {
                None => {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                Some(v) => v,
            }
        };
        Ok(TypedValue::mk_u8(v))
    }

    fn handle_cast_u16(&self, val: TypedValue) -> Result<TypedValue, AbortInfo> {
        let (ty, val, _) = val.decompose();
        let v = if ty.is_u8() {
            val.into_u8() as u16
        } else if ty.is_u16() {
            val.into_u16()
        } else if ty.is_u32() {
            let v = val.into_u32();
            if v > (u16::MAX as u32) {
                return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
            }
            v as u16
        } else if ty.is_u64() {
            let v = val.into_u64();
            if v > (u16::MAX as u64) {
                return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
            }
            v as u16
        } else if ty.is_u128() {
            let v = val.into_u128();
            if v > (u16::MAX as u128) {
                return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
            }
            v as u16
        } else {
            let n = val.into_num();
            match n.to_u16() {
                None => {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                Some(v) => v,
            }
        };
        Ok(TypedValue::mk_u16(v))
    }

    fn handle_cast_u32(&self, val: TypedValue) -> Result<TypedValue, AbortInfo> {
        let (ty, val, _) = val.decompose();
        let v = if ty.is_u8() {
            val.into_u8() as u32
        } else if ty.is_u16() {
            val.into_u16() as u32
        } else if ty.is_u32() {
            val.into_u32()
        } else if ty.is_u64() {
            let v = val.into_u64();
            if v > (u32::MAX as u64) {
                return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
            }
            v as u32
        } else if ty.is_u128() {
            let v = val.into_u128();
            if v > (u32::MAX as u128) {
                return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
            }
            v as u32
        } else {
            let n = val.into_num();
            match n.to_u32() {
                None => {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                Some(v) => v,
            }
        };
        Ok(TypedValue::mk_u32(v))
    }

    fn handle_cast_u64(&self, val: TypedValue) -> Result<TypedValue, AbortInfo> {
        let (ty, val, _) = val.decompose();
        let v = if ty.is_u8() {
            val.into_u8() as u64
        } else if ty.is_u16() {
            val.into_u16() as u64
        } else if ty.is_u32() {
            val.into_u32() as u64
        } else if ty.is_u64() {
            val.into_u64()
        } else if ty.is_u128() {
            let v = val.into_u128();
            if v > (u64::MAX as u128) {
                return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
            }
            v as u64
        } else {
            let n = val.into_num();
            match n.to_u64() {
                None => {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                Some(v) => v,
            }
        };
        Ok(TypedValue::mk_u64(v))
    }

    fn handle_cast_u128(&self, val: TypedValue) -> Result<TypedValue, AbortInfo> {
        let (ty, val, _) = val.decompose();
        let v = if ty.is_u8() {
            val.into_u8() as u128
        } else if ty.is_u16() {
            val.into_u16() as u128
        } else if ty.is_u32() {
            val.into_u32() as u128
        } else if ty.is_u64() {
            val.into_u64() as u128
        } else if ty.is_u128() {
            val.into_u128()
        } else {
            let n = val.into_num();
            match n.to_u128() {
                None => {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                Some(v) => v,
            }
        };
        Ok(TypedValue::mk_u128(v))
    }

    fn handle_cast_u256(&self, val: TypedValue) -> Result<TypedValue, AbortInfo> {
        let (ty, val, _) = val.decompose();
        let v = if ty.is_u8() {
            u256::U256::from(val.into_u8() as u128)
        } else if ty.is_u16() {
            u256::U256::from(val.into_u16() as u128)
        } else if ty.is_u32() {
            u256::U256::from(val.into_u32() as u128)
        } else if ty.is_u64() {
            u256::U256::from(val.into_u64() as u128)
        } else if ty.is_u128() {
            u256::U256::from(val.into_u128())
        } else {
            let n = val.into_num();
            match big_int_to_u256(&n) {
                None => {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                Some(v) => v,
            }
        };
        Ok(TypedValue::mk_u256(v))
    }

    fn handle_binary_arithmetic(
        &self,
        op: &Operation,
        lhs: TypedValue,
        rhs: TypedValue,
        res: &Type,
    ) -> Result<TypedValue, AbortInfo> {
        if cfg!(debug_assertions) {
            assert!(res.is_compatible_for_arithmetic(lhs.get_ty(), rhs.get_ty()));
        }

        let lval = lhs.into_int();
        let rval = rhs.into_int();
        let result = match op {
            Operation::Add => lval + rval,
            Operation::Sub => lval - rval,
            Operation::Mul => lval * rval,
            Operation::Div => {
                if rval.is_zero() {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                lval / rval
            }
            Operation::Mod => {
                if rval.is_zero() {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                lval % rval
            }
            _ => unreachable!(),
        };

        let res_val = if res.is_u8() {
            match result.to_u8() {
                None => {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                Some(v) => TypedValue::mk_u8(v),
            }
        } else if res.is_u16() {
            match result.to_u16() {
                None => {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                Some(v) => TypedValue::mk_u16(v),
            }
        } else if res.is_u32() {
            match result.to_u32() {
                None => {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                Some(v) => TypedValue::mk_u32(v),
            }
        } else if res.is_u64() {
            match result.to_u64() {
                None => {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                Some(v) => TypedValue::mk_u64(v),
            }
        } else if res.is_u128() {
            match result.to_u128() {
                None => {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                Some(v) => TypedValue::mk_u128(v),
            }
        } else if res.is_u256() {
            match big_int_to_u256(&result) {
                None => {
                    return Err(self.sys_abort(StatusCode::ARITHMETIC_ERROR));
                }
                Some(v) => TypedValue::mk_u256(v),
            }
        } else {
            if cfg!(debug_assertions) {
                assert!(res.is_num());
            }
            TypedValue::mk_num(result)
        };
        Ok(res_val)
    }

    fn handle_binary_bitwise(
        &self,
        op: &Operation,
        lhs: TypedValue,
        rhs: TypedValue,
        res: &Type,
    ) -> TypedValue {
        if cfg!(debug_assertions) {
            assert!(res.is_compatible_for_bitwise(lhs.get_ty(), rhs.get_ty()));
        }

        let lval = lhs.into_int();
        let rval = rhs.into_int();
        let result = match op {
            Operation::BitAnd => lval & rval,
            Operation::BitOr => lval | rval,
            Operation::Xor => lval ^ rval,
            _ => unreachable!(),
        };

        if res.is_u8() {
            TypedValue::mk_u8(result.to_u8().unwrap())
        } else if res.is_u16() {
            TypedValue::mk_u16(result.to_u16().unwrap())
        } else if res.is_u32() {
            TypedValue::mk_u32(result.to_u32().unwrap())
        } else if res.is_u64() {
            TypedValue::mk_u64(result.to_u64().unwrap())
        } else if res.is_u128() {
            TypedValue::mk_u128(result.to_u128().unwrap())
        } else {
            if cfg!(debug_assertions) {
                assert!(res.is_u256());
            }
            TypedValue::mk_u256(big_int_to_u256(&result).unwrap())
        }
    }

    fn handle_binary_bitshift(
        &self,
        op: &Operation,
        lhs: TypedValue,
        rhs: TypedValue,
        res: &Type,
    ) -> TypedValue {
        if cfg!(debug_assertions) {
            assert!(res.is_compatible_for_bitshift(lhs.get_ty()));
            assert!(rhs.get_ty().is_u8());
        }
        let rval = rhs.into_u8();
        if lhs.get_ty().is_u8() {
            let lval = lhs.into_u8();
            let result = match op {
                Operation::Shl => lval << rval,
                Operation::Shr => lval >> rval,
                _ => unreachable!(),
            };
            TypedValue::mk_u8(result)
        } else if lhs.get_ty().is_u16() {
            let lval = lhs.into_u16();
            let result = match op {
                Operation::Shl => lval << rval,
                Operation::Shr => lval >> rval,
                _ => unreachable!(),
            };
            TypedValue::mk_u16(result)
        } else if lhs.get_ty().is_u32() {
            let lval = lhs.into_u32();
            let result = match op {
                Operation::Shl => lval << rval,
                Operation::Shr => lval >> rval,
                _ => unreachable!(),
            };
            TypedValue::mk_u32(result)
        } else if lhs.get_ty().is_u64() {
            let lval = lhs.into_u64();
            let result = match op {
                Operation::Shl => lval << rval,
                Operation::Shr => lval >> rval,
                _ => unreachable!(),
            };
            TypedValue::mk_u64(result)
        } else if lhs.get_ty().is_u128() {
            let lval = lhs.into_u128();
            let result = match op {
                Operation::Shl => lval << rval,
                Operation::Shr => lval >> rval,
                _ => unreachable!(),
            };
            TypedValue::mk_u128(result)
        } else {
            if cfg!(debug_assertions) {
                assert!(lhs.get_ty().is_u256());
            }
            let lval = lhs.into_u256();
            let result = match op {
                Operation::Shl => lval << rval,
                Operation::Shr => lval >> rval,
                _ => unreachable!(),
            };
            TypedValue::mk_u256(result)
        }
    }

    fn handle_binary_comparison(
        &self,
        op: &Operation,
        lhs: TypedValue,
        rhs: TypedValue,
        res: &Type,
    ) -> TypedValue {
        if cfg!(debug_assertions) {
            assert!(lhs.get_ty().is_compatible_for_comparison(rhs.get_ty()));
            assert!(res.is_bool());
        }

        let lval = lhs.into_int();
        let rval = rhs.into_int();
        let result = match op {
            Operation::Lt => lval < rval,
            Operation::Le => lval <= rval,
            Operation::Ge => lval >= rval,
            Operation::Gt => lval > rval,
            _ => unreachable!(),
        };
        TypedValue::mk_bool(result)
    }

    fn handle_binary_equality(
        &self,
        op: &Operation,
        lhs: TypedValue,
        rhs: TypedValue,
        res: &Type,
    ) -> TypedValue {
        if cfg!(debug_assertions) {
            assert!(lhs.get_ty().is_compatible_for_equality(rhs.get_ty()));
            assert!(res.is_bool());
        }
        let lval = lhs.get_val();
        let rval = rhs.get_val();
        let result = match op {
            Operation::Eq => lval == rval,
            Operation::Neq => lval != rval,
            _ => unreachable!(),
        };
        TypedValue::mk_bool(result)
    }

    fn handle_unary_boolean(&self, op: &Operation, opv: TypedValue, res: &Type) -> TypedValue {
        if cfg!(debug_assertions) {
            assert!(opv.get_ty().is_bool());
            assert!(res.is_bool());
        }
        let opval = opv.into_bool();
        let result = match op {
            Operation::Not => !opval,
            _ => unreachable!(),
        };
        TypedValue::mk_bool(result)
    }

    fn handle_binary_boolean(
        &self,
        op: &Operation,
        lhs: TypedValue,
        rhs: TypedValue,
        res: &Type,
    ) -> TypedValue {
        if cfg!(debug_assertions) {
            assert!(lhs.get_ty().is_bool());
            assert!(rhs.get_ty().is_bool());
            assert!(res.is_bool());
        }
        let lval = lhs.into_bool();
        let rval = rhs.into_bool();
        let result = match op {
            Operation::And => lval && rval,
            Operation::Or => lval || rval,
            _ => unreachable!(),
        };
        TypedValue::mk_bool(result)
    }

    fn handle_conditional_branch(
        &self,
        cond: TempIndex,
        then_label: Label,
        else_label: Label,
        local_state: &mut LocalState,
    ) {
        let cond_val = local_state.get_value(cond);
        if cfg!(debug_assertions) {
            assert!(cond_val.get_ty().is_bool());
        }
        let label = if cond_val.into_bool() {
            then_label
        } else {
            else_label
        };
        local_state.set_pc(self.code_offset_by_label(label));
    }

    fn handle_abort(&self, index: TempIndex, local_state: &mut LocalState) {
        let val = local_state.get_value(index);
        if cfg!(debug_assertions) {
            assert!(val.get_ty().is_compatible_for_abort_code());
        }
        let abort_code = if val.get_ty().is_u64() {
            val.into_u64()
        } else {
            val.into_num().to_u64().unwrap()
        };
        local_state.terminate_with_abort(self.usr_abort(abort_code));
    }

    fn handle_return(&self, rets: &[TempIndex], local_state: &mut LocalState) {
        if cfg!(debug_assertions) {
            let env = self.target.global_env();
            let decl_ret_types = self.target.get_return_types();
            assert_eq!(rets.len(), decl_ret_types.len());
            for (ret_index, ret_decl_ty) in rets.iter().zip(decl_ret_types) {
                let ret_ty = convert_model_local_type(env, ret_decl_ty, &self.ty_args);
                assert_eq!(&ret_ty, local_state.get_type(*ret_index));
            }
        }

        let ptrs = local_state.collect_pointers();
        let ret_vals = rets
            .iter()
            .map(|index| {
                let val = local_state.get_value(*index);
                // mark mut_ref returns with the pointer trace
                if val.get_ty().is_ref(Some(true)) {
                    val.box_into_mut_ref_ret(&ptrs)
                } else {
                    val
                }
            })
            .collect();
        local_state.terminate_with_return(ret_vals);
    }

    //
    // natives
    //

    fn native_vector_empty(&self) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 1);
        }
        TypedValue::mk_vector(self.ty_args.first().unwrap().clone(), vec![])
    }

    fn native_vector_length(&self, vec_val: TypedValue) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 1);
            // NOTE: this function accepts a value instead of a reference!
            // This is different from the Move native implementation.
            assert_eq!(
                vec_val.get_ty().get_vector_elem(),
                self.ty_args.first().unwrap()
            );
        }
        TypedValue::mk_u64(vec_val.into_vector().len() as u64)
    }

    fn native_vector_borrow(
        &self,
        vec_val: TypedValue,
        elem_val: TypedValue,
    ) -> Result<TypedValue, AbortInfo> {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 1);
            // NOTE: this function accepts a value instead of a reference!
            // This is different from the Move native implementation.
            assert_eq!(
                vec_val.get_ty().get_vector_elem(),
                self.ty_args.first().unwrap()
            );
        }
        let elem_num = elem_val.into_u64() as usize;
        vec_val
            .get_vector_element(elem_num)
            .ok_or_else(|| self.usr_abort(INDEX_OUT_OF_BOUNDS))
    }

    fn native_vector_borrow_mut(
        &self,
        vec_idx: TempIndex,
        vec_val: TypedValue,
        elem_val: TypedValue,
    ) -> Result<TypedValue, AbortInfo> {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 1);
            assert_eq!(
                vec_val.get_ty().get_ref_vector_elem(Some(true)),
                self.ty_args.first().unwrap()
            );
        }
        let elem_num = elem_val.into_u64() as usize;
        vec_val
            .borrow_ref_vector_element(elem_num, true, vec_idx)
            .ok_or_else(|| self.usr_abort(INDEX_OUT_OF_BOUNDS))
    }

    fn native_vector_push_back(
        &self,
        vec_val: TypedValue,
        elem_val: TypedValue,
    ) -> Result<TypedValue, AbortInfo> {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 1);
            assert_eq!(
                vec_val.get_ty().get_ref_vector_elem(Some(true)),
                self.ty_args.first().unwrap()
            );
        }
        // Mirror the Move VM: `push_back` aborts with `EVECTOR_TOO_LONG` (minor status 4)
        // once the vector reaches `vector_len_max` (default 1024 * 1024).
        if let BaseValue::Vector(elems) = vec_val.get_val()
            && elems.len() as u64 >= VECTOR_LEN_MAX
        {
            return Err(self.usr_abort(VECTOR_TOO_LONG));
        }
        Ok(vec_val.update_ref_vector_push_back(elem_val))
    }

    fn native_vector_pop_back(
        &self,
        vec_val: TypedValue,
    ) -> Result<(TypedValue, TypedValue), AbortInfo> {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 1);
            assert_eq!(
                vec_val.get_ty().get_ref_vector_elem(Some(true)),
                self.ty_args.first().unwrap()
            );
        }
        vec_val
            .update_ref_vector_pop_back()
            .ok_or_else(|| self.usr_abort(POP_EMPTY_VEC))
    }

    fn native_vector_destroy_empty(&self, vec_val: TypedValue) -> Result<(), AbortInfo> {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 1);
            assert_eq!(
                vec_val.get_ty().get_vector_elem(),
                self.ty_args.first().unwrap()
            );
        }
        if !vec_val.into_vector().is_empty() {
            return Err(self.usr_abort(DESTROY_NON_EMPTY_VEC));
        }
        Ok(())
    }

    fn native_vector_swap(
        &self,
        vec_val: TypedValue,
        lhs: TypedValue,
        rhs: TypedValue,
    ) -> Result<TypedValue, AbortInfo> {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 1);
            assert_eq!(
                vec_val.get_ty().get_ref_vector_elem(Some(true)),
                self.ty_args.first().unwrap()
            );
        }
        vec_val
            .update_ref_vector_swap(lhs.into_u64() as usize, rhs.into_u64() as usize)
            .ok_or_else(|| self.usr_abort(INDEX_OUT_OF_BOUNDS))
    }

    fn native_signer_borrow_address(&self, signer_val: TypedValue) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
        }
        // NOTE: this function accepts a value instead of a reference!
        // This is different from the Move native implementation.
        let addr = signer_val.into_signer();
        TypedValue::mk_address(addr)
    }

    fn native_hash_sha2_256(&self, bytes_val: TypedValue) -> TypedValue {
        let elem_ty = BaseType::mk_u8();
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
            assert!(bytes_val.get_ty().is_vector_of(&elem_ty));
        }
        let bytes: Vec<_> = bytes_val
            .into_vector()
            .into_iter()
            .map(|e| e.into_u8())
            .collect();
        let digest = sha2_256_of(&bytes);
        let hashed = digest.into_iter().map(TypedValue::mk_u8).collect();
        TypedValue::mk_vector(elem_ty, hashed)
    }

    fn native_hash_sha3_256(&self, bytes_val: TypedValue) -> TypedValue {
        let elem_ty = BaseType::mk_u8();
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
            assert!(bytes_val.get_ty().is_vector_of(&elem_ty));
        }
        let bytes: Vec<_> = bytes_val
            .into_vector()
            .into_iter()
            .map(|e| e.into_u8())
            .collect();
        let digest = sha3_256_of(&bytes);
        let hashed = digest.into_iter().map(TypedValue::mk_u8).collect();
        TypedValue::mk_vector(elem_ty, hashed)
    }

    /// Extract the bytes from a `vector<u8>` value
    fn native_arg_bytes(&self, bytes_val: TypedValue) -> Vec<u8> {
        let elem_ty = BaseType::mk_u8();
        if cfg!(debug_assertions) {
            assert!(bytes_val.get_ty().is_vector_of(&elem_ty));
        }
        bytes_val
            .into_vector()
            .into_iter()
            .map(|e| e.into_u8())
            .collect()
    }

    /// Pack bytes into a `vector<u8>` value
    fn native_bytes_value(&self, bytes: Vec<u8>) -> TypedValue {
        let elems = bytes.into_iter().map(TypedValue::mk_u8).collect();
        TypedValue::mk_vector(BaseType::mk_u8(), elems)
    }

    fn native_hash_keccak256(&self, bytes_val: TypedValue) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
        }
        let bytes = self.native_arg_bytes(bytes_val);
        self.native_bytes_value(keccak256_of(&bytes))
    }

    fn native_hash_blake2b256(&self, bytes_val: TypedValue) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
        }
        let bytes = self.native_arg_bytes(bytes_val);
        self.native_bytes_value(blake2b256_of(&bytes))
    }

    fn native_hash_blake3_256(&self, bytes_val: TypedValue) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
        }
        let bytes = self.native_arg_bytes(bytes_val);
        self.native_bytes_value(blake3_256_of(&bytes))
    }

    fn native_hash_ripemd160(&self, bytes_val: TypedValue) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
        }
        let bytes = self.native_arg_bytes(bytes_val);
        self.native_bytes_value(ripemd160_of(&bytes))
    }

    fn native_signer_address_to_u64(&self, addr_val: TypedValue) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
            assert!(addr_val.get_ty().is_address());
        }
        let bytes = addr_val.into_address().to_vec();
        let mut low = [0u8; 8];
        low.copy_from_slice(&bytes[bytes.len() - 8..]);
        TypedValue::mk_u64(u64::from_be_bytes(low))
    }

    fn native_signer_address_to_bytes(&self, addr_val: TypedValue) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
            assert!(addr_val.get_ty().is_address());
        }
        self.native_bytes_value(addr_val.into_address().to_vec())
    }

    fn native_string_internal_check_utf8(&self, bytes_val: TypedValue) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
        }
        let bytes = self.native_arg_bytes(bytes_val);
        TypedValue::mk_bool(std::str::from_utf8(&bytes).is_ok())
    }

    fn native_string_internal_is_char_boundary(
        &self,
        bytes_val: TypedValue,
        idx_val: TypedValue,
    ) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
        }
        let bytes = self.native_arg_bytes(bytes_val);
        let idx = idx_val.into_u64() as usize;
        let ok = match bytes.get(idx) {
            None => idx == bytes.len(),
            Some(&b) => (b & 0xC0) != 0x80,
        };
        TypedValue::mk_bool(ok)
    }

    fn native_string_internal_sub_string(
        &self,
        bytes_val: TypedValue,
        i_val: TypedValue,
        j_val: TypedValue,
    ) -> Result<TypedValue, AbortInfo> {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
        }
        let bytes = self.native_arg_bytes(bytes_val);
        let i = i_val.into_u64() as usize;
        let j = j_val.into_u64() as usize;
        if j < i {
            // NOTE: this matches the abort code used by the Move VM native implementation
            return Err(self.usr_abort(1));
        }
        Ok(self.native_bytes_value(bytes[i..j].to_vec()))
    }

    fn native_string_internal_index_of(&self, s_val: TypedValue, r_val: TypedValue) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
        }
        let s = self.native_arg_bytes(s_val);
        let r = self.native_arg_bytes(r_val);
        let pos = if r.is_empty() {
            0
        } else {
            s.windows(r.len()).position(|w| w == r).unwrap_or(s.len())
        };
        TypedValue::mk_u64(pos as u64)
    }

    fn native_type_name_get(&self) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 1);
        }
        let type_name = self
            .ty_args
            .first()
            .unwrap()
            .to_move_type_tag()
            .to_canonical_string(/* with_prefix */ false);

        // std::ascii::String
        let string_inst = StructInstantiation {
            ident: StructIdent {
                module: ModuleIdent {
                    address: CORE_CODE_ADDRESS,
                    name: "ascii".to_string(),
                },
                name: "String".to_string(),
            },
            insts: vec![],
            fields: vec![StructField {
                name: "bytes".to_string(),
                ty: BaseType::mk_vector(BaseType::mk_u8()),
            }],
        };
        let string_val = TypedValue::mk_struct(
            string_inst.clone(),
            vec![self.native_bytes_value(type_name.as_bytes().to_vec())],
        );

        // std::type_name::TypeName
        let type_name_inst = StructInstantiation {
            ident: StructIdent {
                module: ModuleIdent {
                    address: CORE_CODE_ADDRESS,
                    name: "type_name".to_string(),
                },
                name: "TypeName".to_string(),
            },
            insts: vec![],
            fields: vec![StructField {
                name: "name".to_string(),
                ty: BaseType::mk_struct(string_inst),
            }],
        };
        TypedValue::mk_struct(type_name_inst, vec![string_val])
    }

    fn native_bcs_to_bytes(&self, object: TypedValue) -> Result<TypedValue, AbortInfo> {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 1);
            object
                .get_ty()
                .is_ref_of(self.ty_args.first().unwrap(), Some(false));
        }
        // The Move VM builds a type layout before serializing and aborts with
        // `VM_MAX_VALUE_DEPTH_REACHED` when the type nesting exceeds `VALUE_DEPTH_MAX`
        // (see `type_to_type_layout_impl` in move-vm-runtime/src/loader.rs). Mirror this
        // behavior so that the stackless interpreter stays in sync with the Move VM.
        if base_type_depth(self.ty_args.first().unwrap()) > VALUE_DEPTH_MAX {
            return Err(self.sys_abort(StatusCode::VM_MAX_VALUE_DEPTH_REACHED));
        }
        object
            .into_bcs_bytes()
            .map(|bytes| {
                let bcs_val = bytes.into_iter().map(TypedValue::mk_u8).collect();
                TypedValue::mk_vector(BaseType::mk_u8(), bcs_val)
            })
            .ok_or_else(|| self.usr_abort(sub_status::NFE_BCS_SERIALIZATION_FAILURE))
    }

    fn native_event_write_to_event_store(
        &self,
        guid_val: TypedValue,
        seq_val: TypedValue,
        msg_val: TypedValue,
        global_state: &mut GlobalState,
    ) {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 1);
            assert_eq!(
                msg_val.get_ty().get_base_type(),
                self.ty_args.first().unwrap()
            );
        }
        let guid = guid_val
            .into_vector()
            .into_iter()
            .map(|e| e.into_u8())
            .collect();
        let seq = seq_val.into_u64();
        global_state.emit_event(guid, seq, msg_val);
    }

    fn native_signature_ed25519_validate_pubkey(&self, key: TypedValue) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
        }
        let bytes: Vec<_> = key.into_vector().into_iter().map(|e| e.into_u8()).collect();
        let valid = ed25519_deserialize_public_key(bytes.as_slice()).is_ok();
        TypedValue::mk_bool(valid)
    }

    fn native_signature_ed25519_signature_verification(
        &self,
        sig_val: TypedValue,
        key_val: TypedValue,
        msg_val: TypedValue,
    ) -> TypedValue {
        if cfg!(debug_assertions) {
            assert_eq!(self.ty_args.len(), 0);
        }

        let sig_bytes: Vec<_> = sig_val
            .into_vector()
            .into_iter()
            .map(|e| e.into_u8())
            .collect();
        let sig = match ed25519_deserialize_signature(sig_bytes.as_slice()) {
            Ok(sig) => sig,
            Err(_) => {
                return TypedValue::mk_bool(false);
            }
        };

        let key_bytes: Vec<_> = key_val
            .into_vector()
            .into_iter()
            .map(|e| e.into_u8())
            .collect();
        let key = match ed25519_deserialize_public_key(key_bytes.as_slice()) {
            Ok(key) => key,
            Err(_) => {
                return TypedValue::mk_bool(false);
            }
        };

        let msg_bytes: Vec<_> = msg_val
            .into_vector()
            .into_iter()
            .map(|e| e.into_u8())
            .collect();
        let verified = ed25519_verify_signature(&key, &sig, &msg_bytes).is_ok();
        TypedValue::mk_bool(verified)
    }

    //
    // utilities
    //

    fn code_offset_by_label(&self, label: Label) -> CodeOffset {
        *self.label_offsets.get(&label).unwrap()
    }

    fn module_location(&self) -> Location {
        let module_id = self.target.module_env().get_verified_module().self_id();
        Location::Module(module_id)
    }

    fn usr_abort(&self, status_code: u64) -> AbortInfo {
        AbortInfo::User(status_code, self.module_location())
    }

    fn sys_abort(&self, status_code: StatusCode) -> AbortInfo {
        AbortInfo::Internal(status_code, self.module_location())
    }

    fn derive_callee_ctxt(
        &self,
        callee_env: &'env FunctionEnv<'env>,
        ty_args: &[MT::Type],
    ) -> FunctionContext<'env> {
        let env = self.target.global_env();
        let callee_target = choose_variant(self.holder, callee_env);

        // check and convert type arguments
        if cfg!(debug_assertions) {
            let callee_ty_params = callee_target.func_env.get_type_parameters();
            // TODO (mengxu) verify type constraints
            assert_eq!(callee_ty_params.len(), ty_args.len());
        }
        let callee_ty_insts: Vec<_> = ty_args
            .iter()
            .map(|ty_arg| convert_model_base_type(env, ty_arg, &self.ty_args))
            .collect();

        // build the context
        FunctionContext::new(self.holder, callee_target, callee_ty_insts, self.level + 1)
    }

    fn prepare_local_state(&self, skip_specs: bool, typed_args: Vec<TypedValue>) -> LocalState {
        let target = &self.target;
        let env = target.global_env();

        // discover and validate local slots
        let param_decls = target.func_env.get_parameters();
        if cfg!(debug_assertions) {
            assert_eq!(param_decls.len(), typed_args.len());
            assert!(param_decls.len() <= target.get_local_count());
        }

        let mut local_slots = vec![];
        for (i, typed_arg) in typed_args.into_iter().enumerate() {
            let name = env
                .symbol_pool()
                .string(target.get_local_name(i))
                .to_string();

            // check that types for local slots is compatible with the declared parameter type
            if cfg!(debug_assertions) {
                let local_ty = target.get_local_type(i);
                let param_decl_ty = &param_decls.get(i).unwrap().1;
                if local_ty != param_decl_ty {
                    assert!(matches!(
                            param_decl_ty,
                            MT::Type::Reference(false, base_ty)
                            if local_ty == base_ty.as_ref()));
                }
                let ty = convert_model_local_type(env, local_ty, &self.ty_args);
                assert_eq!(&ty, typed_arg.get_ty());
            }

            let slot = LocalSlot::new_arg(name, typed_arg);
            local_slots.push(slot);
        }
        for i in param_decls.len()..target.get_local_count() {
            let name = env
                .symbol_pool()
                .string(target.get_local_name(i))
                .to_string();
            let ty = convert_model_local_type(env, target.get_local_type(i), &self.ty_args);
            let slot = LocalSlot::new_tmp(name, ty);
            local_slots.push(slot);
        }
        LocalState::new(local_slots, skip_specs)
    }
}

//**************************************************************************************************
// Entrypoint
//**************************************************************************************************

/// Entrypoint of the interpretation logic
pub fn entrypoint(
    holder: &FunctionTargetsHolder,
    target: FunctionTarget,
    ty_args: &[BaseType],
    typed_args: Vec<TypedValue>,
    skip_specs: bool,
    level: usize,
    global_state: &mut GlobalState,
) -> ExecResult<Vec<TypedValue>> {
    let mut eval_state = EvalState::default();
    let ctxt = FunctionContext::new(holder, target, ty_args.to_vec(), level);
    let local_state =
        ctxt.exec_user_function(skip_specs, typed_args, global_state, &mut eval_state)?;
    let termination = local_state.into_termination_status();
    match termination {
        TerminationStatus::Abort(abort_info) => Err(abort_info),
        TerminationStatus::Return(return_vals) => Ok(return_vals),
        TerminationStatus::None | TerminationStatus::PostAbort(_) => unreachable!(),
    }
}
