use kanari_move_audit::audit_source;
use std::fs;

#[test]
fn audits_real_move_file_clean() {
    let src = fs::read_to_string("move_package/clean.move").unwrap();
    let findings = audit_source(&src, "move_package/clean.move");
    assert!(
        findings.is_empty(),
        "expected no findings, got: {findings:?}"
    );
}

#[test]
fn audits_real_move_file_with_vuln() {
    let src = fs::read_to_string("move_package/vuln.move").unwrap();
    let findings = audit_source(&src, "move_package/vuln.move");
    assert!(
        findings
            .iter()
            .any(|f| f.rule == kanari_move_audit::RuleId::EntryWithoutAuth),
        "expected EntryWithoutAuth finding"
    );
}
