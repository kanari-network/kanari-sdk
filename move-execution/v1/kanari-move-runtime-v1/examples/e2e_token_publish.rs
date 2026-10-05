#![allow(clippy::print_stdout)]
#![allow(clippy::collapsible_if)]
// Example CLI: publish a compiled Move module (james.mv), call an entry function,
// and apply the resulting ChangeSets to `StateManager` to demonstrate E2E flow.
use kanari_move_runtime_v1::move_runtime::EntryFunctionObjectContext;
use kanari_move_runtime_v1::state::StateManager;
use kanari_types::coin::CoinModule;
use kanari_types::transaction::{ObjectInput, ObjectOwnerKind, ObjectRef};
use move_core_types::account_address::AccountAddress as MoveAccountAddress;
use std::env;

mod support;
use support::{cleanup_db_dir, cli_arg, locate_module, publish, read_module, runtime};

const MODULE_FILE: &str = "james.mv";

/// Object-typed entry parameters are bound through declared object inputs
/// (the arg slot itself stays an empty placeholder).
fn mutable_object_input(object_id: &str, owner: &MoveAccountAddress) -> ObjectInput {
    ObjectInput {
        object_ref: ObjectRef::new(object_id.to_string(), None, None),
        owner: Some(ObjectOwnerKind::AddressOwner(owner.to_hex_literal())),
        mutable: true,
    }
}

fn entry_context(
    object_inputs: Vec<ObjectInput>,
    sender: MoveAccountAddress,
    call_tag: u8,
) -> EntryFunctionObjectContext {
    EntryFunctionObjectContext {
        object_inputs,
        sender: Some(sender),
        gas_info: None,
        timestamp: None,
        // Unique per call: object ids are derived from (tx_hash, counter), so
        // reusing a hash across transactions would collide object ids.
        tx_hash: Some(vec![call_tag; 32]),
        persist_runtime_state: true,
        state_overlay: None,
    }
}

fn main() {
    println!("kanari-move-runtime E2E example: publish + call + apply");

    // Simple CLI: [module_path] [function_name] [mint_amount] [recipient_hex]
    let args: Vec<String> = env::args().collect();

    let path = match locate_module(MODULE_FILE, cli_arg(&args, 1)) {
        Ok(path) => path,
        Err(e) => {
            eprintln!("{e:#}");
            return;
        }
    };

    // Initialize runtime with Kanari natives and preload system modules (stdlib + kanari-system).
    // This ensures modules like `0x1::string` are available for linking/verifier.
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

    // Use the module's declared address as the sender so publishing succeeds
    // (the VM requires sender == module address when publishing).
    let publish_sender = *module_id.address();

    // Publish module (no gas accounting here) as the module address
    let publish_cs = match publish(&runtime, bytes, publish_sender) {
        Ok(cs) => cs,
        Err(e) => {
            eprintln!("{e:#}");
            return;
        }
    };

    println!(
        "Publish ChangeSet produced: accounts={}, treasuries={}, token_sets={}",
        publish_cs.owner_deltas.len(),
        publish_cs.treasuries.len(),
        publish_cs.token_balance_sets.len()
    );

    // The runtime runs the module's `init` automatically during publish,
    // so there is no separate setup call anymore.
    println!("Module init ran automatically during publish (runtime-managed).");

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

    // Apply ChangeSets to StateManager to observe state changes (supply, balances)
    let mut state = StateManager::new_in_memory();
    if !publish_cs.is_empty() {
        state
            .apply_changeset(&publish_cs)
            .expect("apply publish changeset");
    }

    // StateManager in DB mode doesn't expose public maps anymore.
    // Use getters or inspect DB for verification.
    println!("State total supply: {:?}", state.total_supply);

    // println!("State token supplies: {:?}", state.token_supplies);
    // println!("State token treasuries: {:?}", state.token_treasuries);

    // Print balances for any accounts touched
    // for (addr, account) in state.accounts.iter() {
    //     if !account.token_balances.is_empty() {
    //         println!(
    //             "OwnerState {:#x} token balances: {:?}",
    //             addr, account.token_balances
    //         );
    //     }
    // }

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

    // Try robust treasury detection by inspecting both `type_` and decoded native data
    for (id, obj) in publish_cs.created_objects.iter() {
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
        for (id, obj) in publish_cs.created_objects.iter() {
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
            // Build args: (&mut TreasuryCap) -> represented by Object ID, amount: u64, recipient: address
            let mint_amount: u64 = 1_000_000_000; // demo amount

            // Use the same address as recipient for demo (publish_sender)
            let recipient_move: MoveAccountAddress = publish_sender;

            println!("Calling Move mint entry to mint {} of {}", mint_amount, tt);

            // Prepare arguments:
            // 1. TreasuryCap object ID (32 bytes)
            // 2. Amount (u64 BCS)
            // 3. Recipient (Address BCS)

            let amount_arg = bcs::to_bytes(&mint_amount).expect("serialize amount");
            let recipient_arg = bcs::to_bytes(&recipient_move).expect("serialize recipient");

            // The mint borrows the TreasuryCap; make sure the runtime can
            // resolve it from its own object storage first.
            if let Some((_, cap_obj)) = publish_cs.created_objects.iter().find(|(id, _)| id == &tid)
            {
                runtime
                    .preload_object_snapshot(
                        &tid,
                        cap_obj.owner,
                        &cap_obj.type_,
                        cap_obj.data.clone(),
                        cap_obj.version,
                    )
                    .unwrap_or_else(|e| eprintln!("Failed to preload TreasuryCap: {:?}", e));
            }

            // Object-typed parameters (TreasuryCap) are bound through
            // object_inputs; their arg slots stay empty placeholders.
            let mint_args = vec![Vec::new(), amount_arg, recipient_arg];
            let mint_inputs = vec![mutable_object_input(&tid, &publish_sender)];

            // Execute mint
            let mint_result = runtime.execute_entry_function_with_object_context_and_persistence(
                &module_id,
                "mint",
                vec![],
                mint_args,
                entry_context(mint_inputs, publish_sender, 1),
            );

            match mint_result {
                Ok(m_cs) => {
                    println!(
                        "Mint successful! Changeset: {} created objects",
                        m_cs.created_objects.len()
                    );
                    for (id, obj) in &m_cs.created_objects {
                        println!(
                            "Mint created object: id={} type={} owner={}",
                            id, obj.type_, obj.owner
                        );
                    }

                    state.apply_changeset(&m_cs).expect("apply mint changeset");

                    // Find the coin object created for recipient
                    let mut coin_id = String::new();
                    for (id, obj) in &m_cs.created_objects {
                        // Check if it's a Coin (simple check)
                        if obj.owner == recipient_move && obj.type_.contains("Coin") {
                            coin_id = id.clone();
                            println!(
                                "Found minted Coin object: {} (amount: {})",
                                coin_id, mint_amount
                            );
                            break;
                        }
                    }

                    if coin_id.is_empty() {
                        println!(
                            "WARNING: Mint did not return a Coin object (maybe transfer native issue). Creating a fake Coin for Transfer demo."
                        );
                        // Create a fake Coin object manually to test Writeback on Transfer
                        let fake_uid_addr = MoveAccountAddress::random();
                        let fake_id_hex = fake_uid_addr.to_hex_literal();
                        coin_id = fake_id_hex.clone();

                        // Construct Coin data: UID (32 bytes) + Balance (8 bytes)
                        let mut coin_data = fake_uid_addr.to_vec();
                        let balance_bytes = bcs::to_bytes(&mint_amount).unwrap();
                        coin_data.extend(balance_bytes);
                    }

                    if !coin_id.is_empty() {
                        // The minted Coin must be resolvable by the runtime
                        // before the transfer/burn demos borrow it.
                        if let Some((_, coin_obj)) =
                            m_cs.created_objects.iter().find(|(id, _)| id == &coin_id)
                        {
                            runtime
                                .preload_object_snapshot(
                                    &coin_id,
                                    coin_obj.owner,
                                    &coin_obj.type_,
                                    coin_obj.data.clone(),
                                    coin_obj.version,
                                )
                                .unwrap_or_else(|e| eprintln!("Failed to preload Coin: {:?}", e));
                        }

                        // --- TRANSFER DEMO ---
                        let transfer_amount = 100_000u64;
                        let receiver_addr_str = "0x1234567890abcdef1234567890abcdef12345678";
                        let receiver =
                            MoveAccountAddress::from_hex_literal(receiver_addr_str).unwrap();

                        println!(
                            "\n--- TRANSFER DEMO: {} to {} ---",
                            transfer_amount, receiver_addr_str
                        );

                        let t_amt_arg =
                            bcs::to_bytes(&transfer_amount).expect("serialize transfer amount");
                        let receiver_arg = bcs::to_bytes(&receiver).expect("serialize receiver");

                        // transfer_amount(coin: &mut Coin, amount: u64, recipient: address)
                        let transfer_args = vec![Vec::new(), t_amt_arg, receiver_arg];
                        let transfer_inputs = vec![mutable_object_input(&coin_id, &publish_sender)];

                        let transfer_res = runtime
                            .execute_entry_function_with_object_context_and_persistence(
                                &module_id,
                                "transfer_amount",
                                vec![],
                                transfer_args,
                                entry_context(transfer_inputs, publish_sender, 2),
                            );

                        match transfer_res {
                            Ok(t_cs) => {
                                println!("Transfer successful!");
                                state
                                    .apply_changeset(&t_cs)
                                    .expect("apply transfer changeset");

                                // Verify Recipient received the coin
                                let mut recipient_coin_found = false;
                                for (id, obj) in &t_cs.created_objects {
                                    if obj.owner == receiver {
                                        println!(
                                            "Recipient {} received object: id={} type={}",
                                            receiver_addr_str, id, obj.type_
                                        );
                                        if obj.type_.contains("Coin") {
                                            // Extract balance from new coin
                                            if obj.data.len() >= 8 {
                                                let bal_bytes = &obj.data[obj.data.len() - 8..];
                                                let val: u64 =
                                                    bcs::from_bytes(bal_bytes).unwrap_or(0);
                                                println!("Recipient Coin Balance: {}", val);
                                                if val == transfer_amount {
                                                    println!(
                                                        "SUCCESS: Recipient received correct amount {}",
                                                        val
                                                    );
                                                } else {
                                                    eprintln!(
                                                        "FAILURE: Recipient received wrong amount {}",
                                                        val
                                                    );
                                                }
                                                recipient_coin_found = true;
                                            }
                                        }
                                    }
                                }
                                if !recipient_coin_found {
                                    println!(
                                        "WARNING: No Coin object found for recipient in ChangeSet created_objects"
                                    );
                                }

                                // Verify Writeback: Check if source Coin version incremented and balance updated
                                if let Ok(Some(updated_coin)) = state.get_object(&coin_id) {
                                    println!(
                                        "Source Coin version after transfer: {}",
                                        updated_coin.version
                                    );

                                    // Check balance (last 8 bytes)
                                    if updated_coin.data.len() >= 8 {
                                        let balance_bytes =
                                            &updated_coin.data[updated_coin.data.len() - 8..];
                                        let balance: u64 =
                                            bcs::from_bytes(balance_bytes).unwrap_or(0);
                                        println!("Source Coin balance after transfer: {}", balance);
                                        let expected = mint_amount - transfer_amount;
                                        if balance == expected {
                                            println!(
                                                "SUCCESS: Source Balance updated correctly to {}",
                                                balance
                                            );
                                        } else {
                                            eprintln!(
                                                "FAILURE: Balance mismatch! Expected {}, got {}",
                                                expected, balance
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => eprintln!("Transfer failed: {:?}", e),
                        }

                        // --- BURN DEMO ---
                        let burn_amount = 50_000u64;
                        println!("\n--- BURN DEMO: {} ---", burn_amount);

                        let b_amt_arg = bcs::to_bytes(&burn_amount).expect("serialize burn amount");
                        // burn_amount(treasury: &mut TreasuryCap, coin: &mut Coin, amount: u64)
                        let burn_args = vec![Vec::new(), Vec::new(), b_amt_arg];
                        let burn_inputs = vec![
                            mutable_object_input(&tid, &publish_sender),
                            mutable_object_input(&coin_id, &publish_sender),
                        ];

                        let burn_res = runtime
                            .execute_entry_function_with_object_context_and_persistence(
                                &module_id,
                                "burn_amount",
                                vec![],
                                burn_args,
                                entry_context(burn_inputs, publish_sender, 3),
                            );
                        match burn_res {
                            Ok(b_cs) => {
                                println!("Burn successful!");
                                state.apply_changeset(&b_cs).expect("apply burn changeset");
                                if let Ok(Some(updated_coin)) = state.get_object(&coin_id) {
                                    println!(
                                        "Source Coin version after burn: {}",
                                        updated_coin.version
                                    );

                                    // Check balance (last 8 bytes)
                                    if updated_coin.data.len() >= 8 {
                                        let balance_bytes =
                                            &updated_coin.data[updated_coin.data.len() - 8..];
                                        let balance: u64 =
                                            bcs::from_bytes(balance_bytes).unwrap_or(0);
                                        println!("Source Coin balance after burn: {}", balance);
                                        let expected = mint_amount - transfer_amount - burn_amount;
                                        if balance == expected {
                                            println!(
                                                "SUCCESS: Balance updated correctly to {}",
                                                balance
                                            );
                                        } else {
                                            eprintln!(
                                                "FAILURE: Balance mismatch! Expected {}, got {}",
                                                expected, balance
                                            );
                                        }
                                    }
                                }
                            }
                            Err(e) => eprintln!("Burn failed: {:?}", e),
                        }
                    }

                    // Final State Dump
                    // println!("\nFinal State - Token Supplies: {:?}", state.token_supplies);
                }
                Err(e) => eprintln!("Mint failed: {:?}", e),
            }
        } else {
            println!("Could not extract token type from TreasuryCap object; skipping auto-mint.");
        }
    } else {
        println!("No TreasuryCap object detected in publish/call ChangeSets.");
    }

    println!("E2E example finished");
    // The runtime owns the RocksDB handle; release it before removing the
    // directory, otherwise Windows still has the files locked.
    drop(runtime);
    cleanup_db_dir();
}
