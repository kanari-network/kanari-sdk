//! Tests for diagnostics false-positive filter.

use kanari_move_analyzer::diagnostics::is_kanari_false_positive;

#[test]
fn invalid_object_sui_filtered() {
    assert!(is_kanari_false_positive(
        "Invalid object 'X'. Structs with the 'key' ability must have 'id: sui::object::UID' as their first field"
    ));
    assert!(!is_kanari_false_positive(
        "Invalid object 'X' for another reason"
    ));
}

#[test]
fn entry_ctx_filtered() {
    assert!(is_kanari_false_positive(
        "Invalid 'entry' parameter type for parameter 'ctx'"
    ));
    assert!(!is_kanari_false_positive(
        "Invalid 'entry' parameter type for parameter 'foo'"
    ));
}

#[test]
fn object_creation_filtered() {
    assert!(is_kanari_false_positive(
        "Invalid object creation without a newly created UID."
    ));
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
