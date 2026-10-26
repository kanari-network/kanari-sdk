#![allow(clippy::print_stdout)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::redundant_closure)]
// E2E example: publish James NFT module, call `setup`, and report created caps
use kanari_move_runtime_v1::move_runtime::EntryFunctionObjectContext;
use kanari_move_runtime_v1::state::StateManager;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind, ObjectRef};
use move_core_types::account_address::AccountAddress as MoveAccountAddress;
use move_core_types::runtime_value::MoveValue;
use serde::Deserialize;
use std::env;

mod support;
use support::{cleanup_db_dir, cli_arg, locate_module, publish, read_module, runtime};

const MODULE_FILE: &str = "nft.mv";

fn main() {
    println!("kanari-move-runtime E2E NFT example: publish + setup");

    let args: Vec<String> = env::args().collect();

    let path = match locate_module(MODULE_FILE, cli_arg(&args, 1)) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{e:#}");
            return;
        }
    };

    let runtime = match runtime() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("{e:#}");
            return;
        }
    };

    let (compiled, bytes) = match read_module(&path) {
        Ok(parsed) => parsed,
        Err(e) => {
            eprintln!("{e:#}");
            return;
        }
    };

    let module_id = compiled.self_id();
    println!("Publishing module {}", module_id);

    let publish_sender = *module_id.address();

    let publish_cs = match publish(&runtime, bytes, publish_sender) {
        Ok(cs) => cs,
        Err(e) => {
            eprintln!("{e:#}");
            return;
        }
    };

    println!(
        "Publish ChangeSet produced: accounts={}, created_objects={}",
        publish_cs.owner_deltas.len(),
        publish_cs.created_objects.len()
    );

    // Look for NftCap and Collection created objects
    let mut found_nftcap: Option<(String, MoveAccountAddress)> = None;
    for (id, obj) in publish_cs.created_objects.iter() {
        if obj.type_.contains("::NftCap") {
            println!("Found NftCap: id={} type={}", id, obj.type_);
            found_nftcap = Some((id.clone(), obj.owner));

            // Preload the NftCap object into the runtime's object storage
            runtime
                .preload_object_snapshot(id, obj.owner, &obj.type_, obj.data.clone(), obj.version)
                .unwrap_or_else(|e| eprintln!("Failed to preload NftCap: {:?}", e));
        } else if obj.type_.contains("::Collection") {
            println!("Found Collection: id={} type={}", id, obj.type_);
            // Preload the Collection object into the runtime's object storage
            runtime
                .preload_object_snapshot(id, obj.owner, &obj.type_, obj.data.clone(), obj.version)
                .unwrap_or_else(|e| eprintln!("Failed to preload Collection: {:?}", e));
        }
    }

    if let Some((nft_id, nftcap_owner)) = found_nftcap {
        println!("Suggested CLI mint command:");
        println!(
            "kanari move call --package {} --module nft --function mint --sender 0x... --args <NftCap_id> <name_bytes> <description_bytes> <number_bytes> <url_bytes> <level_vec> <rarity_vec> <attack_vec> <defense_vec>",
            module_id.address(),
        );

        // Attempt an automatic mint call with default/demo values using only the NftCap.

        // Demo NFT fields for the current mint signature:
        // mint(cap, name, description, url, attribute_keys, attribute_values, number)
        let name_bytes = b"Kari#42".to_vec();
        let desc_bytes = b"Genesis Kari NFT".to_vec();
        let number_bytes = b"42".to_vec();
        let url_bytes = b"https://kanari.example/nft/42.png".to_vec();

        let vec_u8_to_mv = |v: Vec<u8>| -> MoveValue {
            MoveValue::Vector(v.into_iter().map(MoveValue::U8).collect())
        };

        let arg_name = vec_u8_to_mv(name_bytes)
            .simple_serialize()
            .expect("serialize name");
        let arg_desc = vec_u8_to_mv(desc_bytes)
            .simple_serialize()
            .expect("serialize desc");
        let arg_url = vec_u8_to_mv(url_bytes)
            .simple_serialize()
            .expect("serialize url");
        let arg_keys = bcs::to_bytes(&Vec::<String>::new()).expect("serialize attribute keys");
        let arg_values = bcs::to_bytes(&Vec::<String>::new()).expect("serialize attribute values");
        let arg_number = vec_u8_to_mv(number_bytes)
            .simple_serialize()
            .expect("serialize number");

        // Object-typed parameters (NftCap) are bound through object_inputs;
        // their arg slots stay empty placeholders. TxContext is appended by
        // the runtime.
        let mint_args = vec![
            Vec::new(),
            arg_name,
            arg_desc,
            arg_url,
            arg_keys,
            arg_values,
            arg_number,
        ];
        let mint_inputs = vec![ObjectInput {
            object_ref: ObjectRef::new(nft_id.clone(), None, None),
            owner: Some(ObjectOwnerKind::AddressOwner(nftcap_owner.to_hex_literal())),
            mutable: true,
        }];

        println!("Calling Move mint entry with demo args...");

        match runtime.execute_entry_function_with_object_context_and_persistence(
            &module_id,
            "mint",
            vec![],
            mint_args,
            EntryFunctionObjectContext {
                object_inputs: mint_inputs,
                sender: Some(publish_sender),
                gas_info: None,
                timestamp: None,
                // Unique per call: object ids derive from (tx_hash, counter).
                tx_hash: Some(vec![1u8; 32]),
                persist_runtime_state: true,
                state_overlay: None,
            },
        ) {
            Ok(m_cs) => {
                println!(
                    "Mint call ChangeSet produced: accounts={}, created_objects={}, events={}",
                    m_cs.owner_deltas.len(),
                    m_cs.created_objects.len(),
                    m_cs.events.len()
                );

                // Inspect events: print hex and attempt UTF-8 decode
                if !m_cs.events.is_empty() {
                    println!("Events produced by mint:");
                    for ev in m_cs.events.iter() {
                        println!(
                            " - type: {} seq={} key(hex)={} data(hex)={}",
                            ev.type_tag,
                            ev.sequence_number,
                            hex::encode(&ev.key),
                            hex::encode(&ev.event_data)
                        );
                        if let Ok(s) = std::str::from_utf8(&ev.event_data) {
                            println!("   decoded utf8: {}", s);
                        }

                        // Try to parse MintEvent payload via BCS into a Rust struct
                        #[derive(Deserialize, Debug)]
                        struct MintEventPayload {
                            object_id: MoveAccountAddress,
                            name: String,
                            number: String,
                            crestor: MoveAccountAddress,
                        }

                        if let Ok(payload) = bcs::from_bytes::<MintEventPayload>(&ev.event_data) {
                            println!(
                                "   parsed MintEvent: object_id={:#x} name={} number={} crestor={:#x}",
                                payload.object_id, payload.name, payload.number, payload.crestor
                            );
                        }
                        // Heuristic: extract printable ASCII substrings
                        let mut ascii_runs: Vec<String> = Vec::new();
                        let mut cur: Vec<u8> = Vec::new();
                        for &b in ev.event_data.iter() {
                            if b.is_ascii_graphic() || b == b' ' {
                                cur.push(b);
                            } else {
                                if cur.len() >= 3 {
                                    if let Ok(s) = String::from_utf8(cur.clone()) {
                                        ascii_runs.push(s);
                                    }
                                }
                                cur.clear();
                            }
                        }
                        if cur.len() >= 3 {
                            if let Ok(s) = String::from_utf8(cur.clone()) {
                                ascii_runs.push(s);
                            }
                        }
                        if !ascii_runs.is_empty() {
                            println!("   printable substrings: {:?}", ascii_runs);
                        }
                    }
                } else {
                    println!("No events produced by mint.");
                }

                // Persist/apply: apply ChangeSet to StateManager and show state
                let mut state = StateManager::new_in_memory();
                if !m_cs.is_empty() {
                    state.apply_changeset(&m_cs).expect("apply mint changeset");
                }
            }
            Err(e) => {
                eprintln!("mint call failed: {:?}", e);
            }
        }
    } else {
        println!("Could not find NftCap in created objects.");
    }

    println!("E2E NFT example finished");
    // The runtime owns the RocksDB handle; release it before removing the
    // directory, otherwise Windows still has the files locked.
    drop(runtime);
    cleanup_db_dir();
}
