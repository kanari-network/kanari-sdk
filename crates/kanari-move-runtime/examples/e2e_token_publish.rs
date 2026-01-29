#![allow(clippy::print_stdout)]
#![allow(clippy::collapsible_if)]
// Example CLI: publish a compiled Move module (james.mv), call an entry function,
// and apply the resulting ChangeSets to `StateManager` to demonstrate E2E flow.
use kanari_move_runtime::changeset::ChangeSet;
use kanari_move_runtime::move_runtime::MoveRuntime;
use kanari_move_runtime::state::StateManager;
use kanari_types::coin::CoinModule;
use kanari_types::object::UIDRecord;
use kanari_types::tx_context::TxContextRecord;
use move_binary_format::file_format::CompiledModule;
use move_core_types::account_address::AccountAddress as MoveAccountAddress;
use move_core_types::runtime_value::{MoveStruct, MoveValue};
use std::env;
use std::path::Path;
use std::path::PathBuf;

fn find_james_module() -> Option<std::path::PathBuf> {
    let candidates = [
        "james/build/james/bytecode_modules/james.mv",
        "../james/build/james/bytecode_modules/james.mv",
        "../../james/build/james/bytecode_modules/james.mv",
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
    println!("kanari-move-runtime E2E example: publish + call + apply");

    // Simple CLI: [module_path] [function_name] [mint_amount] [recipient_hex]
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
            "Compiled james.mv not found in expected paths; build James first or pass path as first arg."
        );
        return;
    }

    // Initialize runtime with Kanari natives and preload system modules (stdlib + kanari-system).
    // This ensures modules like `0x1::string` are available for linking/verifier.
    let mut runtime = match MoveRuntime::new_with_kanari_natives() {
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

    // Use the module's declared address as the sender so publishing succeeds
    // (the VM requires sender == module address when publishing).
    let publish_sender = *module_id.address();

    // Publish module (no gas accounting here) as the module address
    let publish_cs = match runtime.publish_module(bytes.clone(), publish_sender, None) {
        Ok(cs) => cs,
        Err(e) => {
            eprintln!("publish_module failed: {:?}", e);
            return;
        }
    };

    println!(
        "Publish ChangeSet produced: accounts={}, treasuries={}, token_sets={}",
        publish_cs.account_changes.len(),
        publish_cs.treasuries.len(),
        publish_cs.token_balance_sets.len()
    );

    // Determine function to call (default "setup") and optional mint params from CLI
    let mut function_name = "setup".to_string();
    let mut _cli_mint_amount: Option<u64> = None;
    let mut _cli_recipient: Option<MoveAccountAddress> = None;
    if args.len() > 2 {
        function_name = args[2].clone();
    }
    if args.len() > 3
        && let Ok(v) = args[3].parse::<u64>()
    {
        _cli_mint_amount = Some(v);
    }
    if args.len() > 4
        && let Ok(addr) = MoveAccountAddress::from_hex_literal(&args[4])
    {
        _cli_recipient = Some(addr);
    }

    println!(
        "Attempting to call entry function {}::{}",
        module_id, function_name
    );

    // Build a serialized TxContext to pass as the last arg for entry functions
    let tx_hash = vec![0u8; 32];
    let epoch = 0u64;
    let epoch_timestamp_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .unwrap_or_default()
        .as_millis() as u64;
    let ids_created = 0u64;
    let tx_ctx = TxContextRecord::from_address(
        publish_sender,
        tx_hash.clone(),
        epoch,
        epoch_timestamp_ms,
        ids_created,
    );
    let tx_context_bytes = bcs::to_bytes(&tx_ctx).expect("serialize tx context");

    // Call setup (or specified function) using module address as the sender
    let call_cs = match runtime.execute_entry_function(
        &module_id,
        &function_name,
        vec![],
        vec![tx_context_bytes.clone()],
        Some(publish_sender),
        None,
        None,
    ) {
        Ok(cs) => cs,
        Err(e) => {
            eprintln!("execute_entry_function failed: {:?}", e);
            // proceed — module publish already persisted
            ChangeSet::new()
        }
    };

    println!(
        "Call ChangeSet produced: accounts={}, treasuries={}, token_sets={}",
        call_cs.account_changes.len(),
        call_cs.treasuries.len(),
        call_cs.token_balance_sets.len()
    );

    // Verbose: print created objects, treasuries, token sets from publish and call
    println!("Publish created objects:");
    for (id, obj) in publish_cs.created_objects.iter() {
        println!(" id={} owner={:#x} type={}", id, obj.owner, obj.type_);
    }
    println!("Publish treasuries: {:?}", publish_cs.treasuries);
    println!(
        "Publish token_balance_sets: {:?}",
        publish_cs.token_balance_sets
    );

    println!("Call created objects:");
    for (id, obj) in call_cs.created_objects.iter() {
        println!(" id={} owner={:#x} type={}", id, obj.owner, obj.type_);
    }
    println!("Call treasuries: {:?}", call_cs.treasuries);
    println!("Call token_balance_sets: {:?}", call_cs.token_balance_sets);

    // Apply ChangeSets to StateManager to observe state changes (supply, balances)
    let mut state = StateManager::new();
    if !publish_cs.is_empty() {
        state
            .apply_changeset(&publish_cs)
            .expect("apply publish changeset");
    }
    if !call_cs.is_empty() {
        state
            .apply_changeset(&call_cs)
            .expect("apply call changeset");
    }

    println!("State token supplies: {:?}", state.token_supplies);
    println!("State token treasuries: {:?}", state.token_treasuries);

    // Print balances for any accounts touched
    for (addr, account) in state.accounts.iter() {
        if !account.token_balances.is_empty() {
            println!(
                "Account {:#x} token balances: {:?}",
                addr, account.token_balances
            );
        }
    }

    // If the call produced created objects or treasuries, try to find a TreasuryCap
    let mut found_treasury_id: Option<String> = None;
    // Also print events for debugging
    println!("Publish events:");
    for ev in publish_cs.events.iter() {
        println!(
            " event type={} data(hex)={}",
            ev.type_tag,
            hex::encode(&ev.event_data)
        );
    }
    println!("Call events:");
    for ev in call_cs.events.iter() {
        println!(
            " event type={} data(hex)={}",
            ev.type_tag,
            hex::encode(&ev.event_data)
        );
    }

    // Try robust treasury detection by inspecting both `type_` and decoded native data
    for (id, obj) in publish_cs
        .created_objects
        .iter()
        .chain(call_cs.created_objects.iter())
    {
        // Print raw data hex for inspection
        println!(
            "Created object data hex (first 256): {}",
            if obj.data.len() > 256 {
                hex::encode(&obj.data[0..256])
            } else {
                hex::encode(&obj.data)
            }
        );

        // Attempt to decode native-recorded type string: native stores recipient(32 bytes) || type_str
        let mut decoded_type: Option<String> = None;
        if obj.data.len() > 32 {
            if let Ok(s) = std::str::from_utf8(&obj.data[32..]) {
                decoded_type = Some(s.to_string());
                println!("Decoded native type string: {}", s);
            }
        }

        if obj.type_.contains(CoinModule::TREASURY_CAP_STRUCT)
            || decoded_type
                .as_ref()
                .map(|s| s.contains(CoinModule::TREASURY_CAP_STRUCT))
                .unwrap_or(false)
        {
            println!(
                "Treasury-like object detected: id={} type={} decoded={:?}",
                id, obj.type_, decoded_type
            );
            found_treasury_id = Some(id.clone());
            break;
        }
    }

    if let Some(tid) = found_treasury_id {
        println!("Detected TreasuryCap object id: {}", tid);
        println!("Suggested CLI mint command:");
        println!(
            "kanari move call --package {} --module james --function mint --sender 0x... --args {} 1000000000 0x<recipient_address>",
            module_id.address(),
            tid
        );
        // Attempt an automatic demo mint by constructing a small ChangeSet
        // WARNING: This is a host-side simulated mint (does not call Move `mint`).
        // It demonstrates how StateManager will record supplies and balances.
        // Try to extract the token type from the TreasuryCap object's type string.
        let mut token_type: Option<String> = None;
        for (id, obj) in publish_cs
            .created_objects
            .iter()
            .chain(call_cs.created_objects.iter())
        {
            if id == &tid && obj.type_.contains(CoinModule::TREASURY_CAP_STRUCT) {
                if let Some(start) = obj.type_.find('<') {
                    if let Some(end) = obj.type_.rfind('>') {
                        let inner = &obj.type_[start + 1..end];
                        token_type = Some(inner.to_string());
                    }
                }
                break;
            }
        }

        if let Some(tt) = token_type {
            // Attempt a real Move `mint` call by invoking the `mint` entry function.
            // Build args: (&mut TreasuryCap) -> represented by UIDRecord, amount: u64, recipient: address
            let mint_amount: u64 = 1_000_000_000; // demo amount

            // Use the same address as recipient for demo (publish_sender)
            let recipient_move: MoveAccountAddress = publish_sender;

            // Build Move-style serialized arguments for (&mut TreasuryCap<T>, u64, address)
            // TreasuryCap<T> layout: struct TreasuryCap { id: UID{ addr: address }, total_supply: u64 }
            if let Ok(_uid) = UIDRecord::from_hex_literal(&tid) {
                // Parse the object id address from the hex id string (0x...) for the UID.addr field
                if let Ok(taddr) = MoveAccountAddress::from_hex_literal(&tid) {
                    let uid_mv =
                        MoveValue::Struct(MoveStruct::new(vec![MoveValue::Address(taddr)]));
                    // Assume current total_supply is 0 for freshly created treasury
                    let cap_mv =
                        MoveValue::Struct(MoveStruct::new(vec![uid_mv, MoveValue::U64(0)]));

                    let arg0 = cap_mv.simple_serialize().expect("serialize treasury cap");
                    let arg1 = MoveValue::U64(mint_amount)
                        .simple_serialize()
                        .expect("serialize amount");
                    let arg2 = MoveValue::Address(recipient_move)
                        .simple_serialize()
                        .expect("serialize recipient");

                    // If we built a recipient arg, call `mint_and_transfer`, otherwise call `mint`.
                    println!("Calling Move mint entry to mint {} of {}", mint_amount, tt);
                    // First try calling `mint` (by-value/mutable-ref). If the module
                    // doesn't contain a matching entry, try the `mint_by_address`
                    // wrapper which accepts an `address` for the treasury.
                    let mut mint_args = vec![arg0.clone(), arg1.clone(), arg2.clone()];
                    mint_args.push(tx_context_bytes.clone());
                    let mint_call = runtime.execute_entry_function(
                        &module_id,
                        "mint",
                        vec![],
                        mint_args,
                        Some(publish_sender),
                        None,
                        None,
                    );
                    match mint_call {
                        Ok(m_cs) => {
                            println!(
                                "Mint call ChangeSet produced: accounts={}, treasuries={}, token_sets={}",
                                m_cs.account_changes.len(),
                                m_cs.treasuries.len(),
                                m_cs.token_balance_sets.len()
                            );
                            // If VM produced no token changes, fall back to host-side simulated mint
                            if m_cs.is_empty() {
                                println!(
                                    "VM produced no token ChangeSet — applying host-side simulated mint fallback"
                                );
                                let mut fallback_cs = ChangeSet::new();
                                fallback_cs.add_treasury(publish_sender, tt.clone(), mint_amount);
                                fallback_cs.add_token_balance_set(
                                    recipient_move,
                                    tt.clone(),
                                    mint_amount,
                                );
                                fallback_cs.mint(recipient_move, mint_amount);
                                state
                                    .apply_changeset(&fallback_cs)
                                    .expect("apply simulated mint changeset");
                            } else {
                                // Apply mint changeset to state
                                state.apply_changeset(&m_cs).expect("apply mint changeset");
                            }

                            println!(
                                "After mint - State token supplies: {:?}",
                                state.token_supplies
                            );
                            println!(
                                "After mint - State token treasuries: {:?}",
                                state.token_treasuries
                            );

                            // Print balances touched
                            for (addr, account) in state.accounts.iter() {
                                if !account.token_balances.is_empty() {
                                    println!(
                                        "Account {:#x} token balances: {:?}",
                                        addr, account.token_balances
                                    );
                                }
                            }
                        }
                        Err(e) => {
                            eprintln!("mint call failed: {:?}", e);
                        }
                    }
                }
            } else {
                println!("Could not parse TreasuryCap id into UIDRecord; skipping real mint.");
            }
        } else {
            println!("Could not extract token type from TreasuryCap object; skipping auto-mint.");
        }
    } else {
        println!("No TreasuryCap object detected in publish/call ChangeSets.");
    }

    println!("E2E example finished");
}
