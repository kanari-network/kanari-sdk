use super::*;
use move_vm_types::loaded_data::runtime_types::CachedStructIndex;
use proptest::prelude::*;

#[test]
fn generic_struct_references_are_object_input_candidates() {
    let generic_coin_ref =
        RuntimeType::MutableReference(Box::new(RuntimeType::StructInstantiation(Box::new((
            CachedStructIndex(0),
            vec![RuntimeType::TyParam(0)],
        )))));

    assert_eq!(
        MoveRuntime::object_param_mutability(&generic_coin_ref, |_| true),
        Some(true)
    );
}

#[test]
fn key_struct_values_are_object_input_candidates() {
    let coin_value = RuntimeType::StructInstantiation(Box::new((
        CachedStructIndex(0),
        vec![RuntimeType::TyParam(0)],
    )));

    assert_eq!(
        MoveRuntime::object_param_mutability(&coin_value, |_| true),
        Some(true)
    );
}

#[test]
fn non_struct_references_are_not_object_input_candidates() {
    let vector_ref =
        RuntimeType::Reference(Box::new(RuntimeType::Vector(Box::new(RuntimeType::U8))));

    assert_eq!(
        MoveRuntime::object_param_mutability(&vector_ref, |_| false),
        None
    );
}

#[test]
fn declared_object_inputs_must_match_reference_param_count() {
    let err = MoveRuntime::validate_declared_object_input_bindings(
        &[ObjectInput {
            object_ref: kanari_types::transaction::ObjectRef::new(
                "0x1",
                Some(1),
                Some("d".to_string()),
            ),
            owner: Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
                "0x1".to_string(),
            )),
            mutable: true,
        }],
        &[],
    )
    .expect_err("count mismatch should fail");

    assert!(err.to_string().contains("count mismatch"));
}

#[test]
fn declared_object_inputs_must_match_reference_param_mutability() {
    let err = MoveRuntime::validate_declared_object_input_bindings(
        &[ObjectInput {
            object_ref: kanari_types::transaction::ObjectRef::new(
                "0x1",
                Some(1),
                Some("d".to_string()),
            ),
            owner: Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
                "0x1".to_string(),
            )),
            mutable: false,
        }],
        &[ObjectParamBindingRequirement {
            param_index: 0,
            mutable: true,
        }],
    )
    .expect_err("mutability mismatch should fail");

    assert!(err.to_string().contains("mutability"));
}

#[test]
fn mutable_object_input_can_bind_immutable_reference_param() {
    MoveRuntime::validate_declared_object_input_bindings(
        &[ObjectInput {
            object_ref: kanari_types::transaction::ObjectRef::new(
                "0x1",
                Some(1),
                Some("d".to_string()),
            ),
            owner: Some(kanari_types::transaction::ObjectOwnerKind::AddressOwner(
                "0x1".to_string(),
            )),
            mutable: true,
        }],
        &[ObjectParamBindingRequirement {
            param_index: 0,
            mutable: false,
        }],
    )
    .expect("mutable input should satisfy immutable reference binding");
}

#[test]
fn immutable_object_cannot_bind_mutable_reference_param() {
    let err = MoveRuntime::validate_declared_object_input_bindings(
        &[ObjectInput {
            object_ref: kanari_types::transaction::ObjectRef::new(
                "0x1",
                Some(1),
                Some("d".to_string()),
            ),
            owner: Some(kanari_types::transaction::ObjectOwnerKind::Immutable),
            mutable: true,
        }],
        &[ObjectParamBindingRequirement {
            param_index: 0,
            mutable: true,
        }],
    )
    .expect_err("immutable mutable-ref binding should fail");

    assert!(err.to_string().contains("Immutable object input"));
}

#[test]
fn generic_immutable_reference_is_an_object_input_candidate() {
    let generic_pool_ref = RuntimeType::Reference(Box::new(RuntimeType::StructInstantiation(
        Box::new((CachedStructIndex(1), vec![RuntimeType::TyParam(0)])),
    )));

    assert_eq!(
        MoveRuntime::object_param_mutability(&generic_pool_ref, |_| false),
        Some(false)
    );
}

#[test]
fn multiple_object_inputs_validate_in_parameter_order() {
    let inputs = (0..2)
        .map(|index| ObjectInput {
            object_ref: kanari_types::transaction::ObjectRef::new(
                format!("0x{}", index + 1),
                Some(1),
                Some(format!("digest-{index}")),
            ),
            owner: Some(kanari_types::transaction::ObjectOwnerKind::Shared),
            mutable: true,
        })
        .collect::<Vec<_>>();
    let requirements = vec![
        ObjectParamBindingRequirement {
            param_index: 0,
            mutable: true,
        },
        ObjectParamBindingRequirement {
            param_index: 1,
            mutable: false,
        },
    ];

    MoveRuntime::validate_declared_object_input_bindings(&inputs, &requirements)
        .expect("multiple object inputs should validate in order");
}

#[test]
fn object_input_requires_owner_semantics() {
    let err = MoveRuntime::validate_declared_object_input_bindings(
        &[ObjectInput {
            object_ref: kanari_types::transaction::ObjectRef::new(
                "0x1",
                Some(1),
                Some("d".to_string()),
            ),
            owner: None,
            mutable: true,
        }],
        &[ObjectParamBindingRequirement {
            param_index: 0,
            mutable: true,
        }],
    )
    .expect_err("object input without owner semantics should fail");

    assert!(err.to_string().contains("must declare owner semantics"));
}

fn policy_object_input(index: usize, mutable: bool, owner: Option<ObjectOwnerKind>) -> ObjectInput {
    ObjectInput {
        object_ref: kanari_types::transaction::ObjectRef::new(
            format!("0x{:x}", index + 1),
            Some(index as u64 + 1),
            Some(format!("digest-{index}")),
        ),
        owner,
        mutable,
    }
}

proptest! {
    #![proptest_config(ProptestConfig::with_cases(1024))]
    #[test]
    fn raw_address_mutability_policy_never_allows_cross_owner_owned_objects(
        requested_mutable in any::<bool>(),
        is_coin in any::<bool>(),
    ) {
        let owner = AccountAddress::from_hex_literal("0x1111").unwrap();
        let sender = AccountAddress::from_hex_literal("0x2222").unwrap();
        let object_type = if is_coin {
            format!("0x2::coin::Coin<{}>", kanari_types::gas_coin::GAS_COIN)
        } else {
            "0x48::escrow_like::Marker".to_string()
        };

        prop_assert!(!MoveRuntime::can_mutably_borrow_preloaded_object(
            requested_mutable,
            &object_type,
            &ObjectOwnerKind::AddressOwner(owner.to_hex_literal()),
            owner,
            Some(sender),
            false,
        ));
    }

    #[test]
    fn explicit_cross_owner_policy_allows_only_mutable_non_coin_owned_objects(
        requested_mutable in any::<bool>(),
        is_coin in any::<bool>(),
    ) {
        let owner = AccountAddress::from_hex_literal("0x1111").unwrap();
        let sender = AccountAddress::from_hex_literal("0x2222").unwrap();
        let object_type = if is_coin {
            format!("0x2::coin::Coin<{}>", kanari_types::gas_coin::GAS_COIN)
        } else {
            "0x48::escrow_like::Marker".to_string()
        };

        let allowed = MoveRuntime::can_mutably_borrow_preloaded_object(
            requested_mutable,
            &object_type,
            &ObjectOwnerKind::AddressOwner(owner.to_hex_literal()),
            owner,
            Some(sender),
            true,
        );

        prop_assert_eq!(allowed, requested_mutable && !is_coin);
    }

    #[test]
    fn owner_and_shared_policy_respects_mutable_and_immutable_flags(
        requested_mutable in any::<bool>(),
        use_shared in any::<bool>(),
        use_immutable in any::<bool>(),
    ) {
        let owner = AccountAddress::from_hex_literal("0x1111").unwrap();
        let owner_kind = if use_immutable {
            ObjectOwnerKind::Immutable
        } else if use_shared {
            ObjectOwnerKind::Shared
        } else {
            ObjectOwnerKind::AddressOwner(owner.to_hex_literal())
        };

        let allowed = MoveRuntime::can_mutably_borrow_preloaded_object(
            requested_mutable,
            "0x48::escrow_like::Marker",
            &owner_kind,
            owner,
            Some(owner),
            false,
        );

        prop_assert_eq!(allowed, requested_mutable && !use_immutable);
    }

    #[test]
    fn object_input_binding_policy_matrix_is_strict(
        requirement_mutability in prop::collection::vec(any::<bool>(), 0..8),
        input_mutability in prop::collection::vec(any::<bool>(), 0..10),
        owner_selector in prop::collection::vec(0u8..4, 0..10),
        allow_extra_dependency_inputs in any::<bool>(),
    ) {
        let requirements = requirement_mutability
            .iter()
            .enumerate()
            .map(|(param_index, mutable)| ObjectParamBindingRequirement {
                param_index,
                mutable: *mutable,
            })
            .collect::<Vec<_>>();
        let inputs = input_mutability
            .iter()
            .enumerate()
            .map(|(index, mutable)| {
                let owner = match owner_selector.get(index).copied().unwrap_or(0) {
                    0 => Some(ObjectOwnerKind::AddressOwner(format!("0x{:x}", index + 0x100))),
                    1 => Some(ObjectOwnerKind::Shared),
                    2 => Some(ObjectOwnerKind::Immutable),
                    _ => None,
                };
                policy_object_input(index, *mutable, owner)
            })
            .collect::<Vec<_>>();

        let result = MoveRuntime::validate_object_input_bindings(
            &inputs,
            &requirements,
            allow_extra_dependency_inputs,
        );

        let count_ok = inputs.len() >= requirements.len()
            && (allow_extra_dependency_inputs || inputs.len() == requirements.len());
        let per_param_ok = inputs
            .iter()
            .zip(requirements.iter())
            .all(|(input, requirement)| {
                (!requirement.mutable || input.mutable)
                    && input.owner.is_some()
                    && (!requirement.mutable
                        || !matches!(input.owner, Some(ObjectOwnerKind::Immutable)))
            });

        prop_assert_eq!(result.is_ok(), count_ok && per_param_ok);
    }
}
