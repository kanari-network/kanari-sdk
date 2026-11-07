//! Diagnostics helpers shared between the LSP bin and tests.
//!
//! Stock `move-analyzer` assumes Sui (`sui::object::UID`, `sui::tx_context`).
//! Kanari code uses `kanari_system::object::UID` / `kanari_system::tx_context`
//! at the same address 0x2, which the Kanari compiler accepts but the Sui
//! checker rejects. Suppress those false positives so `kanari move build`
//! (which passes) and the editor agree.

/// True when a compiler diagnostic is a known Sui-only false positive on
/// Kanari code (object UID / entry ctx / object creation).
pub fn is_kanari_false_positive(message: &str) -> bool {
    message.starts_with("Invalid object '") && message.contains("must have 'id: sui::object::UID'")
        || message.starts_with("Invalid 'entry' parameter type for parameter 'ctx'")
        || message.starts_with("Invalid object creation without a newly created UID")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_object_sui() {
        assert!(is_kanari_false_positive(
            "Invalid object 'Pool'. Structs with the 'key' ability must have 'id: sui::object::UID' as their first field",
        ));
    }

    #[test]
    fn invalid_entry_ctx() {
        assert!(is_kanari_false_positive(
            "Invalid 'entry' parameter type for parameter 'ctx'",
        ));
    }

    #[test]
    fn invalid_object_creation() {
        assert!(is_kanari_false_positive(
            "Invalid object creation without a newly created UID.",
        ));
    }

    #[test]
    fn normal_errors_are_not_false_positives() {
        assert!(!is_kanari_false_positive("Unbound type 'TxContext'"));
        assert!(!is_kanari_false_positive(
            "Could not resolve the name 'vector'"
        ));
        assert!(!is_kanari_false_positive(
            "Invalid 'entry' parameter type for parameter 'foo'"
        ));
        assert!(!is_kanari_false_positive(
            "Invalid object creation for something else"
        ));
        assert!(!is_kanari_false_positive(""));
    }
}
