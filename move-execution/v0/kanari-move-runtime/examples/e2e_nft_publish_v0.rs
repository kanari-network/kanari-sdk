#![allow(clippy::print_stdout)]
#![allow(clippy::collapsible_if)]
#![allow(clippy::redundant_closure)]
// E2E example: publish James NFT module (init runs automatically) and report created caps
use kanari_move_runtime_v0::move_runtime::MoveRuntime;
use kanari_move_runtime_v0::state::StateManager;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress as MoveAccountAddress;
use serde::Deserialize;
use std::env;
use std::path::Path;
use std::path::PathBuf;

fn find_james_module() -> Option<std::path::PathBuf> {
    let candidates = [
        "example_move/james/build/james/bytecode_modules/nft.mv",
        "../example_move/james/build/james/bytecode_modules/nft.mv",
        "../../example_move/james/build/james/bytecode_modules/nft.mv",
    ];
    for p in candidates.iter() {
        let path = Path::new(p);
        if path.exists() {
            return Some(path.to_path_buf());
        }
    }
    None
}

fn main() {
    println!("kanari-move-runtime-v0 E2E NFT example: publish (init runs automatically)");

    let args: Vec<String> = env::args().collect();
    let mut path = match find_james_module() {
        Some(p) => p,
        None => PathBuf::new(),
    };

    if args.len() > 1 {
        let p = PathBuf::from(&args[1]);
        if p.exists() {
            path = p;
        }
    }

    if path.as_os_str().is_empty() {
        eprintln!(
            "Compiled nft.mv not found in expected paths; build James first or pass path as first arg."
        );
        return;
    }

    let runtime = match MoveRuntime::new_with_kanari_natives() {
        Ok(r) => r,
        Err(e) => {
            eprintln!("Failed to init MoveRuntime: {:?}", e);
            return;
        }
    };

    let bytes = match std::fs::read(&path) {
        Ok(b) => b,
        Err(e) => {
            eprintln!("Failed to read {}: {}", path.display(), e);
            return;
        }
    };

    let compiled = match CompiledModule::deserialize_with_defaults(&bytes) {
        Ok(c) => c,
        Err(e) => {
            eprintln!("Failed to deserialize module bytes: {:?}", e);
            return;
        }
    };

    let module_id = compiled.self_id();
    println!("Publishing module {}", module_id);

    let publish_sender = *module_id.address();

    let publish_cs = match runtime.publish_module(bytes.clone(), publish_sender, None, None) {
        Ok(cs) => cs,
        Err(e) => {
            eprintln!("publish_module failed: {:?}", e);
            return;
        }
    };

    println!(
        "Publish ChangeSet produced: accounts={}, created_objects={}",
        publish_cs.account_changes.len(),
        publish_cs.created_objects.len()
    );

    // The runtime runs the module's `init` automatically during publish,
    // so there is no separate setup call anymore.
    println!("Module init ran automatically during publish (runtime-managed).");

    // Look for NftCap and Collection created objects
    let mut found_nftcap: Option<String> = None;
    let mut _found_collection: Option<String> = None;
    for (id, obj) in publish_cs.created_objects.iter() {
        if obj.type_.contains("::NftCap") {
            println!("Found NftCap: id={} type={}", id, obj.type_);
            found_nftcap = Some(id.clone());

            // Preload the NftCap object into the runtime's object storage
            runtime
                .preload_object_snapshot(id, obj.owner, &obj.type_, obj.data.clone(), obj.version)
                .unwrap_or_else(|e| eprintln!("Failed to preload NftCap: {:?}", e));
        } else if obj.type_.contains("::Collection") {
            println!("Found Collection: id={} type={}", id, obj.type_);
            _found_collection = Some(id.clone());

            // Preload the Collection object into the runtime's object storage
            runtime
                .preload_object_snapshot(id, obj.owner, &obj.type_, obj.data.clone(), obj.version)
                .unwrap_or_else(|e| eprintln!("Failed to preload Collection: {:?}", e));
        }
    }

    if let Some(nft_id) = found_nftcap {
        println!("Suggested CLI mint command:");
        println!(
            "kanari move call --package {} --module nft --function mint --sender 0x... --args <NftCap_id> <name_bytes> <description_bytes> <number_bytes> <url_bytes> <level_vec> <rarity_vec> <attack_vec> <defense_vec>",
            module_id.address(),
        );

        // Attempt an automatic mint call with default/demo values using only the NftCap.

        // Parse object id into address for UID.addr field
        if let Ok(naddr) = MoveAccountAddress::from_hex_literal(&nft_id) {
            // For mutable references like &mut NftCap, we only need to pass the object ID
            // The Move VM will look up the full object from storage
            let arg0 = naddr.into_bytes().to_vec(); // Just the 32-byte address, not the full struct

            // Plain BCS encoding matches the Move parameter layouts directly:
            // `vector<u8>` is a length-prefixed byte vector and
            // `vector<String>` a length-prefixed vector of length-prefixed
            // UTF-8 strings.
            let arg1 = bcs::to_bytes(&b"Kari#42".to_vec()).expect("serialize name");
            let arg2 = bcs::to_bytes(&b"Genesis Kari NFT".to_vec()).expect("serialize desc");
            let arg3 = bcs::to_bytes(&b"https://kanari.example/nft/42.png".to_vec())
                .expect("serialize url");
            let arg4 = bcs::to_bytes(&vec!["level".to_string(), "rarity".to_string()])
                .expect("serialize attribute keys");
            let arg5 = bcs::to_bytes(&vec!["1".to_string(), "common".to_string()])
                .expect("serialize attribute values");
            let arg6 = bcs::to_bytes(&b"42".to_vec()).expect("serialize number");

            // Build args: cap, name, description, url, attribute_keys,
            // attribute_values, number (TxContext is appended automatically).
            let mint_args = vec![arg0, arg1, arg2, arg3, arg4, arg5, arg6];

            println!("Calling Move mint entry with demo args...");
            // Debug: print arg lengths and hex to help diagnose deserialization failures
            for (i, a) in mint_args.iter().enumerate() {
                println!(" mint arg[{}] len={} hex={} ", i, a.len(), hex::encode(a));
            }

            match runtime.execute_entry_function(
                &module_id,
                "mint",
                vec![],
                mint_args,
                Some(publish_sender),
                None,
                None,
            ) {
                Ok(m_cs) => {
                    println!(
                        "Mint call ChangeSet produced: accounts={}, created_objects={}, events={}",
                        m_cs.account_changes.len(),
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

                            match bcs::from_bytes::<MintEventPayload>(&ev.event_data) {
                                Ok(payload) => {
                                    println!(
                                        "   parsed MintEvent: object_id={:#x} name={} number={} crestor={:#x}",
                                        payload.object_id,
                                        payload.name,
                                        payload.number,
                                        payload.crestor
                                    );
                                }
                                Err(_) => {
                                    // fallback: show printable substrings (already handled below)
                                }
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

                    // StateManager DB mode doesn't expose public maps
                    // println!(
                    //     "After mint - State token supplies: {:?}",
                    //     state.token_supplies
                    // );
                    // println!(
                    //     "After mint - State token treasuries: {:?}",
                    //     state.token_treasuries
                    // );
                    // for (addr, account) in state.accounts.iter() {
                    //     if !account.token_balances.is_empty() {
                    //         println!(
                    //             "Account {:#x} token balances: {:?}",
                    //             addr, account.token_balances
                    //         );
                    //     }
                    // }
                }
                Err(e) => {
                    eprintln!("mint call failed: {:?}", e);
                }
            }
        } else {
            println!("Failed to parse NftCap id as address; skipping auto-mint.");
        }
    } else {
        println!("Could not find NftCap in created objects.");
    }

    println!("E2E NFT example finished");
}
