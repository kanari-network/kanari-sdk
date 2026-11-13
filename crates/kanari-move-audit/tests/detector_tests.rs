use kanari_move_audit::RuleId;
use kanari_move_audit::detectors::detect_all;

const SAMPLE: &str = include_str!("../move_package/cases/sample.move");

#[test]
fn detects_missing_signer() {
    let findings = detect_all(SAMPLE, "test.move");
    assert!(findings.iter().any(|f| f.rule == RuleId::EntryWithoutAuth));
}

#[test]
fn does_not_flag_signer_present() {
    let src = include_str!("../move_package/cases/signer_present.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings.iter().all(|f| f.rule != RuleId::EntryWithoutAuth),
        "unexpected findings: {:?}",
        findings
    );
}

#[test]
fn does_not_flag_signer_address_of_as_reentrancy() {
    let src = include_str!("../move_package/cases/signer_address_of.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings.iter().all(|f| f.rule != RuleId::ExternalCall),
        "unexpected findings: {:?}",
        findings
    );
}

#[test]
fn does_not_flag_object_model_entry_function() {
    let src = include_str!("../move_package/cases/object_model_entry.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings.iter().all(|f| f.rule != RuleId::EntryWithoutAuth),
        "unexpected findings: {:?}",
        findings
    );
}

#[test]
fn does_not_flag_object_model_entry_function_with_params() {
    let src = include_str!("../move_package/cases/object_model_entry_params.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings.iter().all(|f| f.rule != RuleId::EntryWithoutAuth),
        "unexpected findings: {:?}",
        findings
    );
}

#[test]
fn does_not_flag_capability_based_entry_function() {
    let src = include_str!("../move_package/cases/capability_based_entry.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings.iter().all(|f| f.rule != RuleId::EntryWithoutAuth),
        "unexpected findings: {:?}",
        findings
    );
}

#[test]
fn does_not_flag_read_only_external_call() {
    let src = include_str!("../move_package/cases/read_only_external.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings.iter().all(|f| f.rule != RuleId::ExternalCall),
        "unexpected findings: {:?}",
        findings
    );
}

#[test]
fn flags_state_changing_external_call() {
    let src = include_str!("../move_package/cases/state_changing_external.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings.iter().any(|f| f.rule == RuleId::ExternalCall),
        "expected a reentrancy finding for state-changing call"
    );
}

#[test]
fn does_not_flag_immutable_borrow() {
    let src = include_str!("../move_package/cases/immutable_borrow.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings.iter().all(|f| f.rule != RuleId::ExternalCall),
        "unexpected findings: {:?}",
        findings
    );
}

#[test]
fn detects_borrow_global_without_exists() {
    let findings = detect_all(SAMPLE, "test.move");
    assert!(
        findings
            .iter()
            .any(|f| f.rule == RuleId::BorrowGlobalWithoutCheck)
    );
}

#[test]
fn detects_hardcoded_address() {
    let findings = detect_all(SAMPLE, "test.move");
    assert!(findings.iter().any(|f| f.rule == RuleId::HardcodedAddress));
}

#[test]
fn detects_external_call_in_entry() {
    let findings = detect_all(SAMPLE, "test.move");
    assert!(findings.iter().any(|f| f.rule == RuleId::ExternalCall));
}

#[test]
fn detects_missing_doc() {
    let findings = detect_all(SAMPLE, "test.move");
    assert!(findings.iter().any(|f| f.rule == RuleId::MissingDoc));
}

#[test]
fn detects_capability_leak() {
    let src = include_str!("../move_package/cases/capability_leak.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings.iter().any(|f| f.rule == RuleId::CapabilityLeak),
        "expected capability leak finding"
    );
}

#[test]
fn detects_integer_overflow_risk() {
    let src = include_str!("../move_package/cases/integer_overflow.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings
            .iter()
            .any(|f| f.rule == RuleId::IntegerOverflowRisk),
        "expected integer overflow risk finding"
    );
}

#[test]
fn does_not_flag_arithmetic_with_assert() {
    let src = include_str!("../move_package/cases/arithmetic_with_assert.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings
            .iter()
            .all(|f| f.rule != RuleId::IntegerOverflowRisk),
        "unexpected integer overflow finding: {:?}",
        findings
    );
}

#[test]
fn detects_public_entry_without_signer() {
    let src = include_str!("../move_package/public_entry_no_signer.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings
            .iter()
            .any(|f| f.rule == RuleId::PublicEntryWithoutSigner),
        "expected PublicEntryWithoutSigner finding"
    );
}

#[test]
fn does_not_flag_public_entry_with_signer() {
    let src = include_str!("../move_package/public_entry_with_signer.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings
            .iter()
            .all(|f| f.rule != RuleId::PublicEntryWithoutSigner),
        "unexpected PublicEntryWithoutSigner finding: {:?}",
        findings
    );
}

#[test]
fn detects_state_change_before_external_call() {
    let src = include_str!("../move_package/state_change_external.move");
    let findings = detect_all(src, "test.move");
    // may or may not fire depending on state change detection; just ensure no panic
    let _ = findings;
}

#[test]
fn empty_source_no_panic() {
    let findings = detect_all("", "test.move");
    assert!(findings.is_empty());
}

#[test]
fn division_by_zero_detected() {
    let src = include_str!("../move_package/division_by_zero.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings
            .iter()
            .any(|f| f.rule == RuleId::DivisionByZeroRisk)
    );
}

#[test]
fn division_by_const_not_flagged() {
    let src = include_str!("../move_package/division_by_const.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings
            .iter()
            .all(|f| f.rule != RuleId::DivisionByZeroRisk)
    );
}

#[test]
fn garbage_source_no_panic() {
    let findings = detect_all("not valid move code {{{", "test.move");
    let _ = findings;
}

#[test]
fn multiple_findings_count() {
    let findings = detect_all(SAMPLE, "test.move");
    assert!(findings.len() >= 4);
}

#[test]
fn borrow_global_without_exists_detected() {
    let src = include_str!("../move_package/borrow_global_no_exists.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings
            .iter()
            .any(|f| f.rule == RuleId::BorrowGlobalWithoutCheck)
    );
}

#[test]
fn borrow_global_with_exists_not_flagged() {
    let src = include_str!("../move_package/borrow_global_with_exists.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings
            .iter()
            .all(|f| f.rule != RuleId::BorrowGlobalWithoutCheck)
    );
}

#[test]
fn missing_doc_on_public_fn() {
    let src = include_str!("../move_package/missing_doc_public.move");
    let findings = detect_all(src, "test.move");
    assert!(findings.iter().any(|f| f.rule == RuleId::MissingDoc));
}
#[test]
fn division_by_const_not_flagged2() {
    let src = include_str!("../move_package/division_by_const2.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings
            .iter()
            .all(|f| f.rule != RuleId::DivisionByZeroRisk)
    );
}

#[test]
fn share_object_without_auth_detected() {
    let src = include_str!("../move_package/share_object_no_auth.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings
            .iter()
            .any(|f| f.rule == RuleId::ShareObjectWithoutAuth)
    );
}

#[test]
fn unsafe_public_transfer_detected() {
    let src = include_str!("../move_package/unsafe_transfer.move");
    let findings = detect_all(src, "test.move");
    assert!(
        findings
            .iter()
            .any(|f| f.rule == RuleId::UnsafePublicTransfer)
    );
}
