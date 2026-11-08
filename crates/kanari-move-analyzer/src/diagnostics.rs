//! Diagnostics helpers shared between the LSP bin and tests.
//!
//! Stock `move-analyzer` assumes Sui (`sui::object::UID`, `sui::tx_context`).
//! Kanari code uses `kanari_system::object::UID` / `kanari_system::tx_context`
//! at the same address 0x2, which the Kanari compiler accepts but the Sui
//! checker rejects. Suppress those false positives so `kanari move build`
//! (which passes) and the editor agree.

/// True when a compiler diagnostic is a known Sui-only false positive on
/// Kanari code (object UID / object creation). Entry-param false positives
/// live in `is_kanari_false_positive_with_file` (they need the param type,
/// which is only in the file — no name-based fallback here).
pub fn is_kanari_false_positive(message: &str) -> bool {
    message.starts_with("Invalid object '") && message.contains("must have 'id: sui::object::UID'")
        || message.starts_with("Invalid object creation without a newly created UID")
}

/// Param name of an `Invalid 'entry' parameter type` diagnostic, if any.
pub fn entry_param_name(message: &str) -> Option<String> {
    const PREFIX: &str = "Invalid 'entry' parameter type for parameter '";
    let rest = message.strip_prefix(PREFIX)?;
    let end = rest.find('\'')?;
    let name = &rest[..end];
    if name.is_empty() {
        None
    } else {
        Some(name.to_string())
    }
}

/// File-aware filter: suppress an `Invalid 'entry' parameter type`
/// diagnostic when the flagged param really is `... TxContext`. The
/// param's *name* is irrelevant — only its *type* is checked, so any
/// name works without setting anything up. The check looks at the
/// diagnostic line (and the next line when the type is split), so a
/// `TxContext` on a neighbouring function does not leak into this
/// diagnostic. Non-`TxContext` entry params (real Sui-rule violations)
/// are *not* filtered here.
pub fn is_kanari_false_positive_with_file(message: &str, file_text: &str, line: usize) -> bool {
    if is_kanari_false_positive(message) {
        return true;
    }
    let Some(name) = entry_param_name(message) else {
        return false;
    };
    let lines: Vec<&str> = file_text.lines().collect();
    if line >= lines.len() {
        return false;
    }
    // The diagnostic points at the param name; `TxContext` is on that
    // line in all normal formatting, or on the next line if the author
    // split `name:` and its type. Only look forward, never backward,
    // so a `TxContext` on a neighbouring function cannot leak in.
    let mut window = lines[line].to_string();
    if !window.contains("TxContext")
        && let Some(next) = lines.get(line + 1)
    {
        window.push('\n');
        window.push_str(next);
    }
    // The param name must appear as an identifier in the window and the
    // window must declare a `TxContext` type (possibly qualified,
    // e.g. `&mut tx_context::TxContext`).
    contains_identifier(&window, &name) && window.contains("TxContext")
}

fn contains_identifier(haystack: &str, needle: &str) -> bool {
    let mut from = 0;
    while let Some(rel) = haystack[from..].find(needle) {
        let pos = from + rel;
        let before_ok = haystack[..pos]
            .chars()
            .next_back()
            .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
        let after_ok = haystack[pos + needle.len()..]
            .chars()
            .next()
            .is_none_or(|c| !c.is_ascii_alphanumeric() && c != '_');
        if before_ok && after_ok {
            return true;
        }
        from = pos + needle.len();
    }
    false
}

/// Diagnostics that are only a warning on Kanari (init runs once on publish).
/// Should be yellow, not red.
pub fn is_kanari_warning(message: &str) -> bool {
    message.starts_with("Invalid 'init' function declaration")
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
        // Message-only no longer filters entry diagnostics — the param type
        // decides, so `with_file` is required (see tests below). A bare
        // message must never claim a false positive (no name fallback).
        assert!(!is_kanari_false_positive(
            "Invalid 'entry' parameter type for parameter 'ctx'",
        ));
        assert!(!is_kanari_false_positive(
            "Invalid 'entry' parameter type for parameter '_ctx'",
        ));
        assert!(!is_kanari_false_positive(
            "Invalid 'entry' parameter type for parameter 'my_ctx'",
        ));
        // ctx2 is a different param — not a tx-context name
        assert!(!is_kanari_false_positive(
            "Invalid 'entry' parameter type for parameter 'ctx2'",
        ));
        assert!(!is_kanari_false_positive(
            "Invalid 'entry' parameter type for parameter '_ctx2'",
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
        assert!(!is_kanari_warning("Unbound type 'TxContext'"));
    }

    #[test]
    fn init_is_warning_not_false_positive() {
        assert!(is_kanari_warning("Invalid 'init' function declaration"));
        assert!(!is_kanari_warning(
            "Invalid 'entry' parameter type for parameter 'ctx'"
        ));
    }

    #[test]
    fn entry_param_name_extraction() {
        assert_eq!(
            entry_param_name("Invalid 'entry' parameter type for parameter 'ctx'"),
            Some("ctx".to_string())
        );
        assert_eq!(
            entry_param_name("Invalid 'entry' parameter type for parameter '_ctx'"),
            Some("_ctx".to_string())
        );
        assert_eq!(
            entry_param_name("Invalid 'entry' parameter type for parameter 'my_ctx'"),
            Some("my_ctx".to_string())
        );
        assert_eq!(entry_param_name("Unbound type 'TxContext'"), None);
        assert_eq!(entry_param_name(""), None);
    }

    #[test]
    fn entry_any_name_with_tx_context_type_is_filtered() {
        let msg = "Invalid 'entry' parameter type for parameter 'my_ctx'";
        let code = "\
module m::n {
    public entry fun f(
        a: address,
        my_ctx: &mut TxContext
    ) {}
}
";
        // line is 0-based; `my_ctx` is on line 3
        assert!(is_kanari_false_positive_with_file(msg, code, 3));
        // other names with TxContext type on the same line shape
        for name in [
            "ctx",
            "_ctx",
            "__ctx",
            "context",
            "_context",
            "x",
            "_x",
            "my_ctx",
            "woof",
            "THE_CTX",
            "a",
            "sender",
            "_unused_thing",
            "ctx1",
        ] {
            let code = format!(
                "module m::n {{\n    public entry fun f({name}: &mut TxContext) {{}}\n}}\n"
            );
            let msg = format!("Invalid 'entry' parameter type for parameter '{name}'");
            assert!(
                is_kanari_false_positive_with_file(&msg, &code, 1),
                "should filter {name} with TxContext type"
            );
        }
        // qualified type `&mut tx_context::TxContext` also counts
        let code = "module m::n {\n    fun f(c: &tx_context::TxContext) {}\n}\n";
        assert!(is_kanari_false_positive_with_file(
            "Invalid 'entry' parameter type for parameter 'c'",
            code,
            1
        ));
    }

    #[test]
    fn entry_split_type_on_next_line_is_filtered() {
        let msg = "Invalid 'entry' parameter type for parameter 'my_ctx'";
        let code = "\
module m::n {
    public entry fun f(
        my_ctx:
            &mut TxContext
    ) {}
}
";
        assert!(is_kanari_false_positive_with_file(msg, code, 2));
    }

    #[test]
    fn entry_non_tx_context_param_is_not_filtered() {
        // `foo: address` invalid entry type is a real error — must stay.
        let msg = "Invalid 'entry' parameter type for parameter 'foo'";
        let code = "module m::n {\n    fun f(foo: address) {}\n}\n";
        assert!(!is_kanari_false_positive_with_file(msg, code, 1));
        // ctx-named param with a non-TxContext type must also stay (this is
        // why the old name-only fallback was removed).
        let msg2 = "Invalid 'entry' parameter type for parameter 'bar'";
        let code2 = "module m::n {\n    fun f(bar: u64) {}\n}\n";
        assert!(!is_kanari_false_positive_with_file(msg2, code2, 1));
        // `my_ctx: address` (ctx-looking name, wrong type) must not be
        // silently hidden either.
        let msg3 = "Invalid 'entry' parameter type for parameter 'my_ctx'";
        let code3 = "module m::n {\n    fun f(my_ctx: address) {}\n}\n";
        assert!(!is_kanari_false_positive_with_file(msg3, code3, 1));
    }

    #[test]
    fn entry_tx_context_on_other_function_does_not_leak() {
        let msg = "Invalid 'entry' parameter type for parameter 'foo'";
        let code = "\
module m::n {
    public entry fun f(a: address, ctx: &mut TxContext) {}
    public entry fun g(foo: bool) {}
}
";
        // `g`'s `foo` is on line 2; ctx on line 1 must not leak into the window
        assert!(!is_kanari_false_positive_with_file(msg, code, 2));
    }
}
