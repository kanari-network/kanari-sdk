//! Tests for diagnostics false-positive filter.

use kanari_move_analyzer::diagnostics::{
    is_kanari_false_positive, is_kanari_false_positive_with_file,
};

#[test]
fn invalid_object_sui_filtered() {
    assert!(is_kanari_false_positive(
        "Invalid object 'X'. Structs with the 'key' ability must have 'id: sui::object::UID' as their first field"
    ));
    assert!(!is_kanari_false_positive("Invalid object 'X' for another reason"));
}

#[test]
fn entry_ctx_filtered() {
    // No name fallback — entry params are filtered only when the file shows
    // a `TxContext` type, whatever the param is named.
    for name in ["ctx", "_ctx", "__ctx", "my_ctx", "context", "x", "_x"] {
        let code =
            format!("module m::n {{\n    public entry fun f({name}: &mut TxContext) {{}}\n}}\n");
        let msg = format!("Invalid 'entry' parameter type for parameter '{name}'");
        assert!(
            is_kanari_false_positive_with_file(&msg, &code, 1),
            "should filter {name} with TxContext type"
        );
    }
    // Message alone (no file context) must not filter any entry diagnostic.
    for name in ["ctx", "_ctx", "foo", "_ctx2"] {
        let msg = format!("Invalid 'entry' parameter type for parameter '{name}'");
        assert!(!is_kanari_false_positive(&msg), "message-only must not filter {name}");
    }
    // Non-TxContext entry types stay visible.
    let code = "module m::n {\n    fun f(foo: address) {}\n}\n";
    assert!(!is_kanari_false_positive_with_file(
        "Invalid 'entry' parameter type for parameter 'foo'",
        code,
        1
    ));
    let code = "module m::n {\n    fun f(doh: u64) {}\n}\n";
    assert!(!is_kanari_false_positive_with_file(
        "Invalid 'entry' parameter type for parameter 'doh'",
        code,
        1
    ));
}

#[test]
fn object_creation_filtered() {
    assert!(is_kanari_false_positive("Invalid object creation without a newly created UID."));
    assert!(!is_kanari_false_positive("Invalid object creation"));
}

#[test]
fn real_errors_not_filtered() {
    for msg in [
        "Unbound type 'TxContext' in current scope",
        "Could not resolve the name 'move-analyzer'",
        "Unbound function 'create_order_book_with_decimals'",
        "Could not infer this type. Try adding an annotation",
        "",
        "Invalid object 'Pool'. Structs with the 'key' ability must have 'id: kanari_system::object::UID' as their first field",
    ] {
        assert!(!is_kanari_false_positive(msg), "should not filter: {msg}");
    }
}
