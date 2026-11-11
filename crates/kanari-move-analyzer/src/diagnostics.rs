//! Diagnostics helpers shared between the LSP bin and tests.
//!
//! Stock `move-analyzer` assumes Sui (`sui::object::UID`, `sui::tx_context`).
//! Kanari code uses `kanari_system::object::UID` / `kanari_system::tx_context`
//! at the same address 0x2, which the Kanari compiler accepts but the Sui
//! checker rejects. Suppress those false positives so `kanari move build`
//! (which passes) and the editor agree.

/// True when a compiler diagnostic is a known Sui-only false positive on
/// Kanari code (object UID / object creation, OTW drop-only rule,
/// `init`-OTW order). Entry-param false positives live in
/// `is_kanari_false_positive_with_file` (they need the param type,
/// which is only in the file — no name-based fallback here).
/// `kanari move build` passes on all of these; the Kanari compiler is
/// the source of truth, so the editor must not report them at all.
pub fn is_kanari_false_positive(message: &str) -> bool {
    message.starts_with("Invalid object '") && message.contains("must have 'id: sui::object::UID'")
        || message.starts_with("Invalid object creation without a newly created UID")
        // Kanari compiles OTW-like test structs with key/store/drop fine
        // (`kanari move build` passes). The `sui_mode` checker in
        // `move-analyzer` defaults flavor=Sui and flags them — suppress,
        // the Kanari compiler is the source of truth.
        || message.starts_with("Invalid one-time witness declaration")
        // Sui requires `init` first param to be the OTW; Kanari allows
        // the OTW (or visibility patterns) after it. `kanari move build`
        // accepts these, so this is not an error on Kanari at all.
        || message.starts_with("Invalid 'init' function declaration")
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

/// Entry-param check: assert the param at `line` is declared with a type
/// that is one of `expected` simple identifiers (base name; may be raised
/// qualified as `mod::Name`, possibly split across lines). Never checks
/// the param's *name* — any name is accepted.
fn entry_param_has_type(message: &str, file_text: &str, line: usize, expected: &[&str]) -> bool {
    let Some(name) = entry_param_name(message) else {
        return false;
    };
    let lines: Vec<&str> = file_text.lines().collect();
    if line >= lines.len() {
        return false;
    }
    let line_text = lines[line];
    if !contains_identifier(line_text, &name) {
        return false;
    }
    if line_text.contains(expected[0]) {
        return true;
    }
    // Multi-line param (type on the next line): type token may carry its
    // own prefix (`&`, `&mut`, module qualifier) — check tokens only.
    let Some(next) = lines.get(line + 1) else {
        return false;
    };
    let hay = next;
    expected.iter().any(|e| hay.contains(*e))
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
    entry_param_has_type(message, file_text, line, &["TxContext"])
}

/// True when the flagged entry param is the object-address identifier type
/// (`ID` from `kanari_system::object`, possibly qualified as
/// `object::ID`). Kanari's framework uses `ID` as a plain object id in
/// several `public entry` functions and `kanari move build` accepts them
/// all, so a Sui-default analyzer flagging them red disagrees with the
/// Kanari compiler. (The Kanari compiler is the source of truth.)
pub fn is_kanari_entry_id_param(message: &str, file_text: &str, line: usize) -> bool {
    entry_param_has_type(message, file_text, line, &["ID"])
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

/// Diagnostics that never surface in the editor as Kanari errors —
/// older callers downgrade these to warnings, but Kanari accepts them
/// (`kanari move build` passes), so they are filtered entirely.
pub fn is_kanari_warning(message: &str) -> bool {
    // kept for API compatibility: init declarations are now filtered
    // entirely by `is_kanari_false_positive`.
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
    fn invalid_otw_test_fixture() {
        // `kanari move build` accepts these (they are test-only OTWs); the
        // Sui-default checker flags the drop-only rule — suppress.
        assert!(is_kanari_false_positive(
            "Invalid one-time witness declaration"
        ));
    }

    #[test]
    fn entry_id_param_is_filtered() {
        // `kiosk::list_item` shape: `item_id: ID,` on one line.
        let msg = "Invalid 'entry' parameter type for parameter 'item_id'";
        let code = "\
module m::kiosk {
    public entry fun list_item(
        kiosk: &mut Kiosk,
        item_id: ID,
        price: u64
    ) {}
}
";
        assert!(is_kanari_entry_id_param(msg, code, 3));
        let code2 = "module m::kiosk {\n    public entry fun list(item_id: ID) {}\n}\n";
        assert!(is_kanari_entry_id_param(
            "Invalid 'entry' parameter type for parameter 'item_id'",
            code2,
            1
        ));
    }

    #[test]
    fn entry_id_qualified_object_id() {
        let msg = "Invalid 'entry' parameter type for parameter 'item_id'";
        let code = "module m::kiosk {\n    public entry fun list(item_id: object::ID) {}\n}\n";
        assert!(is_kanari_entry_id_param(msg, code, 1));
        // a real invalid entry type must NOT be filtered
        assert!(!is_kanari_entry_id_param(
            "Invalid 'entry' parameter type for parameter 'foo'",
            "module m::m {\n    public entry fun f(foo: u64) {}\n}\n",
            1
        ));
        // TxContext-looking entry param is the other case — not an ID param
        assert!(!is_kanari_entry_id_param(
            "Invalid 'entry' parameter type for parameter 'ctx'",
            "module m::m {\n    public entry fun f(ctx: &TxContext) {}\n}\n",
            1
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
    fn init_unused_warning_is_never_filtered() {
        // W09008: `fun init` has no in-source callers (the runtime invokes
        // it on publish), so `kanari move build` reports it as a *warning*
        // — yellow — unless the module/member carries
        // `#[allow(unused_function)]` (compiler-side warning filter). The
        // editor must surface exactly that warning, never swallow it and
        // never escalate it: it passes every Kanari filter untouched.
        let msg =
            "The non-'public', non-'entry' function 'init' is never called. Consider removing it.";
        let file = "module m::m {\n    fun init() {}\n}\n";
        assert!(!is_kanari_false_positive(msg));
        assert!(!is_kanari_false_positive_with_file(msg, file, 1));
        assert!(!is_kanari_entry_id_param(msg, file, 1));
        assert!(!is_kanari_warning(msg), "already a warning, no downgrade");
        // Non-init private functions keep the same treatment.
        let helper = "The non-'public', non-'entry' function 'helper' is never called. Consider removing it.";
        assert!(!is_kanari_false_positive(helper));
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

    #[test]
    fn entry_every_case() {
        // Every case that Sui-default checker flags but Kanari accepts:
        // (param name, param type, should-filter?)
        let cases: &[(&str, &str, bool)] = &[
            // Direct TxContext family (any name)
            ("ctx", "&mut TxContext", true),
            ("ctx", "&TxContext", true),
            ("x", "&mut tx_context::TxContext", true),
            ("_y", "&TxContext", true),
            // ID family (any name)
            ("item_id", "ID", true),
            ("pid", "ID", true),
            ("x", "object::ID", true),
            ("o", "kanari_system::object::ID", true),
            // Multi-line split type
            ("my_ctx", "\n        &mut TxContext", true),
            // Real entry violations — must never be hidden
            ("kiosk", "&mut Kiosk", false),
            ("amount", "u64", false),
            ("flag", "bool", false),
            ("cap", "&KioskOwnerCap<K>", false),
        ];
        for (name, ty, filtered) in cases {
            let params_decl = if ty.starts_with('\n') {
                format!("{name}:")
            } else {
                format!("{name}:{ty}")
            };
            let code = if ty.starts_with('\n') {
                format!(
                    "module m::n {{\n    public entry fun f({params_decl}\n        &mut TxContext\n    ) {{}}\n}}\n"
                )
            } else {
                format!("module m::n {{\n    public entry fun f({name}: {ty}) {{}}\n}}\n")
            };
            let msg = format!("Invalid 'entry' parameter type for parameter '{name}'");
            assert_eq!(
                is_kanari_false_positive_with_file(&msg, &code, 1)
                    || is_kanari_entry_id_param(&msg, &code, 1),
                *filtered,
                "case {name}:{ty}"
            );
        }
        // OUT OF RANGE / missing line
        assert!(!is_kanari_false_positive_with_file(
            "Invalid 'entry' parameter type for parameter 'x'",
            "module m::n {\n    public entry fun f(x: &mut TxContext) {}\n}\n",
            99
        ));
        assert!(!is_kanari_entry_id_param(
            "Invalid 'entry' parameter type for parameter 'x'",
            "",
            0
        ));
    }
}
