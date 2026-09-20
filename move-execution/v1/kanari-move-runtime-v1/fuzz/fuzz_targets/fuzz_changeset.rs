// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use kanari_move_runtime_v1::changeset::{ChangeSet, CreatedObject, StateAccessSet};
use kanari_move_runtime_v1::state::StateManager;
use kanari_types::balance::BalanceRecord;
use kanari_types::transaction::{ObjectChange, ObjectChangeKind, ObjectOwnerKind};
use libfuzzer_sys::fuzz_target;
use move_core_types::account_address::AccountAddress;

/// Arbitrary AccountAddress generator
#[derive(Debug, Arbitrary)]
struct ArbitraryAccountAddress {
    bytes: [u8; 32],
}

impl From<ArbitraryAccountAddress> for AccountAddress {
    fn from(arb: ArbitraryAccountAddress) -> Self {
        AccountAddress::new(arb.bytes)
    }
}

/// Arbitrary ObjectOwnerKind generator
#[derive(Debug, Arbitrary)]
enum ArbitraryObjectOwnerKind {
    AddressOwner,
    ObjectOwner,
    SharedOwner,
    Immutable,
}

impl From<ArbitraryObjectOwnerKind> for ObjectOwnerKind {
    fn from(arb: ArbitraryObjectOwnerKind) -> Self {
        match arb {
            ArbitraryObjectOwnerKind::AddressOwner => ObjectOwnerKind::AddressOwner(
                AccountAddress::from_hex_literal("0x11111111111111111111111111111111")
                    .unwrap_or_default(),
            ),
            ArbitraryObjectOwnerKind::ObjectOwner => {
                ObjectOwnerKind::ObjectOwner("0x22222222222222222222222222222222".to_string())
            }
            ArbitraryObjectOwnerKind::SharedOwner => ObjectOwnerKind::SharedOwner {
                initial_shared_version: 1,
            },
            ArbitraryObjectOwnerKind::Immutable => ObjectOwnerKind::Immutable,
        }
    }
}

/// Arbitrary CreatedObject generator
#[derive(Debug, Arbitrary)]
struct ArbitraryCreatedObject {
    owner_bytes: [u8; 32],
    owner_kind: ArbitraryObjectOwnerKind,
    type_str: String,
    data: Vec<u8>,
    version: u64,
}

impl From<ArbitraryCreatedObject> for CreatedObject {
    fn from(arb: ArbitraryCreatedObject) -> Self {
        Self {
            owner: AccountAddress::new(arb.owner_bytes),
            owner_kind: arb.owner_kind.into(),
            uid: None,
            id: None,
            type_: arb.type_str,
            data: arb.data,
            version: arb.version,
        }
    }
}

/// Arbitrary ChangeSet generator
#[derive(Debug, Arbitrary)]
struct ArbitraryChangeSet {
    objects: Vec<ArbitraryCreatedObject>,
    owner_bytes: [u8; 32],
    token_type: String,
    balance: u64,
    state_access_reads: Vec<Vec<u8>>,
    state_access_writes: Vec<Vec<u8>>,
}

impl From<ArbitraryChangeSet> for ChangeSet {
    fn from(arb: ArbitraryChangeSet) -> Self {
        let mut cs = ChangeSet::new();

        for obj in arb.objects {
            let created: CreatedObject = obj.into();
            cs.created_objects.push((
                format!(
                    "0x{}",
                    kanari_crypto::hash_data_blake3(&created.data)
                        .iter()
                        .take(8)
                        .map(|b| format!("{:02x}", b))
                        .collect::<String>()
                ),
                created,
            ));
        }

        let owner = AccountAddress::new(arb.owner_bytes);
        cs.add_token_balance_set(owner, arb.token_type, arb.balance);

        let mut access_set = StateAccessSet::default();
        for key in arb.state_access_reads {
            access_set.read(key);
        }
        for key in arb.state_access_writes {
            access_set.write(key);
        }
        cs.state_access_set = access_set;

        cs
    }
}

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);

    // Generate random ChangeSet
    if let Ok(arb_cs) = ArbitraryChangeSet::arbitrary(&mut unstructured) {
        let cs: ChangeSet = arb_cs.into();

        // Try to apply to a StateManager - should not panic
        let mut state = StateManager::new_in_memory();
        let _ = state.apply_changeset(&cs);

        // Verify internal consistency
        for (obj_id, created) in &cs.created_objects {
            let _ = obj_id;
            let _ = created.digest();
            let _ = created.object_ref(obj_id);
        }

        // Verify StateAccessSet operations
        let access_clone = cs.state_access_set.clone();
        let _conflicts = access_clone.conflicts_with(&cs.state_access_set);
    }
});
