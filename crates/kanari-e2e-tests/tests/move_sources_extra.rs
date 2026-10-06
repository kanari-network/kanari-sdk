//! Structural checks over the Move sources that are not published/executed
//! by `move_compiled_runtime`.
//!
//! These assert on *declarations* (`public entry fun foo(`) rather than bare
//! names, so a mention inside a doc comment, a call site, or a similarly
//! prefixed function (e.g. `add_simple_liquidity` matching `add_liquidity`)
//! cannot satisfy them.

fn read(file: &str) -> String {
    std::fs::read_to_string(format!("move_package/sources/{file}"))
        .unwrap_or_else(|e| panic!("failed to read {file}: {e}"))
}

/// Asserts that `file` declares `declaration` verbatim as code.
fn assert_declares(file: &str, src: &str, declaration: &str) {
    assert!(
        src.contains(declaration),
        "{file} must declare `{declaration}`"
    );
}

/// Asserts that every `declaration` appears in `file`.
fn assert_all(file: &str, src: &str, declarations: &[&str]) {
    for declaration in declarations {
        assert_declares(file, src, declaration);
    }
}

/// The module header must be the first code line, so a doc comment naming the
/// module cannot stand in for the real declaration.
fn assert_module_header(file: &str, src: &str, expected: &str) {
    let header = src
        .lines()
        .map(str::trim)
        .find(|line| !line.is_empty() && !line.starts_with("//") && !line.starts_with('#'))
        .unwrap_or_else(|| panic!("{file} contains no code"));
    assert!(
        header.starts_with(expected) && header.ends_with('{'),
        "{file} must open with its module declaration, found `{header}`"
    );
}

#[test]
fn extra_move_sources_exist() {
    let sources_dir = std::path::Path::new("move_package/sources");
    assert!(sources_dir.exists());

    let files = [
        "move_e2e_storage.move",
        "move_e2e_time.move",
        "move_e2e_nft.move",
        "move_e2e_pool.move",
        "move_e2e_registry.move",
    ];

    for f in &files {
        let path = sources_dir.join(f);
        assert!(
            path.exists(),
            "Move source should exist: {}",
            path.display()
        );
        let content = std::fs::read_to_string(&path)
            .unwrap_or_else(|e| panic!("failed to read {}: {e}", path.display()));
        assert!(
            content.len() > 50,
            "{} is suspiciously short ({} bytes)",
            path.display(),
            content.len()
        );
        assert!(
            content.contains("public entry fun "),
            "{} must expose at least one entry function",
            path.display()
        );
    }
}

#[test]
fn extra_move_modules_have_correct_names() {
    let expected = [
        (
            "move_e2e_storage.move",
            "module kanari_e2e_tests::move_e2e_storage",
        ),
        (
            "move_e2e_time.move",
            "module kanari_e2e_tests::move_e2e_time",
        ),
        ("move_e2e_nft.move", "module kanari_e2e_tests::move_e2e_nft"),
        (
            "move_e2e_pool.move",
            "module kanari_e2e_tests::move_e2e_pool",
        ),
        (
            "move_e2e_registry.move",
            "module kanari_e2e_tests::move_e2e_registry",
        ),
    ];

    for (file, header) in expected {
        let src = read(file);
        assert_module_header(file, &src, header);
        // A single `module` declaration per file keeps the header check honest.
        assert_eq!(
            src.matches("module kanari_e2e_tests::").count(),
            1,
            "{file} must declare exactly one module"
        );
    }
}

#[test]
fn move_files_contain_expected_logic() {
    let pool = read("move_e2e_pool.move");
    assert_all(
        "move_e2e_pool.move",
        &pool,
        &[
            // Declarations: the bare-name check would let
            // `add_simple_liquidity` satisfy `add_liquidity`.
            "public fun create_pool<A, B>(",
            "public fun add_liquidity<A, B>(",
            "public fun remove_liquidity<A, B>(",
            "public entry fun create_simple_pool(",
            "public entry fun add_simple_liquidity(",
            "public entry fun remove_simple_liquidity(",
            // Types and the guards that make the pool usable.
            "public struct Pool<phantom A, phantom B> has key, store",
            "public struct SimplePool has key, store",
            "const E_INSUFFICIENT_SHARES: u64 = 600;",
            "const E_EMPTY_POOL: u64 = 601;",
            "assert!(reserve_a > 0 && reserve_b > 0, E_EMPTY_POOL);",
            "pool.total_shares = pool.total_shares + shares;",
            "assert!(pool.total_shares >= shares, E_INSUFFICIENT_SHARES);",
        ],
    );

    let nft = read("move_e2e_nft.move");
    assert_all(
        "move_e2e_nft.move",
        &nft,
        &[
            "public entry fun e2e_create_collection(",
            "public entry fun e2e_mint_nft(",
            "public entry fun transfer_nft(nft: NFT, recipient: address)",
            "public entry fun e2e_burn_nft(nft: NFT)",
            "public struct NFT has key, store",
            "public struct NFTCollection has key, store",
            "const E_NOT_OWNER: u64 = 500;",
            "assert!(collection.owner == sender, E_NOT_OWNER);",
            "creator: sender",
            "vector::push_back(&mut collection.nfts, nft_addr);",
        ],
    );

    let storage = read("move_e2e_storage.move");
    assert_all(
        "move_e2e_storage.move",
        &storage,
        &[
            "public entry fun set_value(",
            "public fun get_value(store: &KeyValueStore, key: vector<u8>): option::Option<vector<u8>>",
            "public entry fun remove_value(",
            "public entry fun update_blob(",
            "public struct KeyValueStore has key, store",
            "public struct DataBlob has key, store",
            "const E_NOT_OWNER: u64 = 300;",
            "const E_BAD_VALUE: u64 = 301;",
            "assert!(store.owner == tx_context::sender(ctx), E_NOT_OWNER);",
            "blob.version = blob.version + 1;",
            // Parallel arrays must stay in step: the key is appended alongside
            // its value, and the pair is removed together.
            "vector::push_back(&mut store.keys, key_str);",
            "vector::push_back(&mut store.values, value);",
            "vector::remove(&mut store.keys, pos);",
            "vector::remove(&mut store.values, pos);",
        ],
    );

    let registry = read("move_e2e_registry.move");
    assert_all(
        "move_e2e_registry.move",
        &registry,
        &[
            "public fun create_registry(ctx: &mut TxContext): address",
            "public entry fun register_name(",
            "public fun lookup(registry: &Registry, name: vector<u8>): option::Option<address>",
            "public fun create_entry(",
            "public entry fun update_entry(",
            "public struct Registry has key, store",
            "public struct Entry has key, store",
            "const E_NOT_OWNER: u64 = 700;",
            "const E_BAD_LOOKUP: u64 = 701;",
            "assert!(registry.owner == tx_context::sender(ctx), E_NOT_OWNER);",
            "assert!(entry.owner == tx_context::sender(ctx), E_NOT_OWNER);",
            // Re-registration overwrites the address in place rather than
            // appending a duplicate name.
            "*vector::borrow_mut(&mut registry.addresses, pos) = addr;",
        ],
    );

    let time = read("move_e2e_time.move");
    assert_all(
        "move_e2e_time.move",
        &time,
        &[
            "public entry fun update_record(",
            "public fun create_record(ctx: &mut TxContext): address",
            "public fun is_fresh(",
            "public fun clock_address(): address",
            "public struct TimestampRecord has key, store",
            "const E_NOT_OWNER: u64 = 400;",
            "const E_BAD_CLOCK: u64 = 401;",
            "const E_BAD_TIME: u64 = 402;",
            "assert!(record.owner == tx_context::sender(ctx), E_NOT_OWNER);",
            "clock::timestamp_ms_by_address(clock_address())",
            // Freshness is inclusive of `threshold_ms` and rejects clocks
            // earlier than the last update.
            "now_ms >= record.last_updated && now_ms - record.last_updated <= threshold_ms",
            "record.last_updated = now_ms;",
        ],
    );
}
