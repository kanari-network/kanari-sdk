// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

#![no_main]

use arbitrary::{Arbitrary, Unstructured};
use kanari_move_runtime_v2::changeset::{ChangeSet, CreatedObject, StateAccessSet};
use kanari_move_runtime_v2::state::StateManager;
use kanari_types::transaction::ObjectOwnerKind;
use libfuzzer_sys::fuzz_target;
use move_core_types::account_address::AccountAddress;

fn default_owner_address() -> AccountAddress {
    AccountAddress::from_hex_literal("0x11111111111111111111111111111111")
        .unwrap_or_else(|_| AccountAddress::new([0x11u8; 32]))
}

fn object_id_for(data: &[u8]) -> String {
    let hash = kanari_crypto::hash_data_blake3(data);
    let mut object_id = String::from("0x");
    for byte in hash.iter().take(8) {
        object_id.push_str(&format!("{:02x}", byte));
    }
    object_id
}

/// Arbitrary ObjectOwnerKind generator (AddressOwner | Shared | Immutable).
#[derive(Debug, Arbitrary)]
enum ArbitraryObjectOwnerKind {
    AddressOwner,
    Shared,
    Immutable,
}

impl From<ArbitraryObjectOwnerKind> for ObjectOwnerKind {
    fn from(arb: ArbitraryObjectOwnerKind) -> Self {
        match arb {
            ArbitraryObjectOwnerKind::AddressOwner => {
                ObjectOwnerKind::AddressOwner(default_owner_address().to_hex_literal())
            }
            ArbitraryObjectOwnerKind::Shared => ObjectOwnerKind::Shared,
            ArbitraryObjectOwnerKind::Immutable => ObjectOwnerKind::Immutable,
        }
    }
}

/// Arbitrary CreatedObject generator.
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

/// Arbitrary ChangeSet generator.
#[derive(Debug, Arbitrary)]
struct ArbitraryChangeSet {
    objects: Vec<ArbitraryCreatedObject>,
    owner_bytes: [u8; 32],
    token_type: String,
    balance: u64,
    access_reads: Vec<Vec<u8>>,
    access_writes: Vec<Vec<u8>>,
}

fuzz_target!(|data: &[u8]| {
    let mut unstructured = Unstructured::new(data);
    let Ok(arb) = ArbitraryChangeSet::arbitrary(&mut unstructured) else {
        return;
    };

    let mut cs = ChangeSet::new();
    for obj in arb.objects {
        let created: CreatedObject = obj.into();
        let object_id = object_id_for(&created.data);
        cs.created_objects.push((object_id, created));
    }
    cs.add_token_balance_set(
        AccountAddress::new(arb.owner_bytes),
        arb.token_type,
        arb.balance,
    );

    // Access manifests are built through the public reads/writes sets.
    let mut access = StateAccessSet::default();
    for key in arb.access_reads {
        access.reads.insert(key);
    }
    for key in arb.access_writes {
        access.writes.insert(key);
    }

    // Applying a fuzzed changeset must never panic.
    let mut state = StateManager::new_in_memory();
    let _ = state.apply_changeset(&cs);

    // Object invariants: digest and object_ref stay consistent with the payload.
    for (object_id, created) in &cs.created_objects {
        let digest = created.digest();
        assert!(!digest.is_empty(), "digest must never be empty");
        let object_ref = created.object_ref(object_id);
        assert_eq!(object_ref.object_id, *object_id);
        assert_eq!(object_ref.version, Some(created.version));
        assert_eq!(object_ref.digest.as_ref(), Some(&digest));
    }

    // Conflict detection is symmetric, and an empty manifest never conflicts.
    let derived = cs.deterministic_access_set();
    assert_eq!(
        access.conflicts_with(&derived),
        derived.conflicts_with(&access),
        "conflicts_with must be symmetric"
    );
    assert!(
        !StateAccessSet::default().conflicts_with(&access),
        "an empty access set never conflicts"
    );
});
