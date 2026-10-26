//! Negative-compile guard for the `kanari_pet` upgrade economy.
//!
//! `pet::Stats` holds `base_cost_usd`, `power_per_level` and `max_level`, and
//! both `pet::upgrade` and `pet::mint_with_level` accept a `&Stats`. If a
//! package could construct its own `Stats`, it would choose the economics — a
//! one-unit price, a million power per level, no ceiling — and buy arbitrary
//! progression for pennies. That was a real hole: `new_stats` was `public`.
//!
//! The fix makes `new_stats` `#[test_only]`, so `init` is the only producer and
//! the shared object is the single source of truth. `#[test_only]` is invisible
//! to the in-package test suite — it still exists there — so the control is
//! only observable from outside, which is exactly what this fixture checks.
//!
//! The fixture package `pet_stats_forgery` therefore MUST NOT compile. The
//! compile runs in a child process for the same reason the deny-list fixtures
//! do: `report_diagnostics` calls `std::process::exit(1)`, so an in-process
//! attempt would kill the test binary and report success for a leaked fixture.

use anyhow::Result;
use move_package::BuildConfig;
use std::path::Path;

/// `kanari_pet::pet::new_stats` must be unreachable from another package.
///
/// Regression test for a confirmed exploit: while `new_stats` was `public`, a
/// caller built `Stats { base_cost_usd: 1, power_per_level: 1_000_000,
/// max_level: 9_999 }` and bought a million power for one micro-USD.
#[test]
fn pet_stats_forgery_is_locked() -> Result<()> {
    assert_fixture_does_not_compile(
        "pet_stats_forgery",
        "pet_stats_forgery_is_locked",
        "outside package compiled a call to kanari_pet::pet::new_stats; \
         upgrade economics are caller-controlled again, so any package can set \
         a one-unit price and an unbounded power_per_level and buy arbitrary \
         progression for pennies",
    )
}

/// Assert a checked-in negative fixture fails to compile.
///
/// Runs the compile in a child process (this same test binary re-executed with
/// a sentinel argument) and judges it by exit status, because
/// `report_diagnostics` terminates the process on failure.
///
/// The sentinel must not start with `--`: libtest rejects unknown leading
/// options, which would make every child exit non-zero and let a leaked fixture
/// pass unnoticed. `is_child` is therefore also asserted in the parent, so a
/// broken child invocation fails loudly instead of silently reporting success.
fn assert_fixture_does_not_compile(
    package: &str,
    test_name: &str,
    failure_message: &str,
) -> Result<()> {
    let sentinel = format!("{package}-compile-child");
    let package_dir = Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("tests")
        .join("move_packages")
        .join(package);

    if is_child(&sentinel) {
        // Child: exit 0 only if the fixture compiles (which means it leaked).
        // Report the failure reason on stderr so a broken fixture (bad
        // dependency path, unresolvable package) is distinguishable from a
        // genuine visibility rejection. `report_diagnostics` terminates the
        // process itself, so this only fires for pre-diagnostic failures.
        match BuildConfig::default().compile_package(&package_dir, &mut Vec::new()) {
            Ok(_) => std::process::exit(0),
            Err(e) => {
                eprintln!("fixture compile error: {e:?}");
                std::process::exit(1);
            }
        }
    }

    let status = std::process::Command::new(std::env::current_exe()?)
        .arg(sentinel)
        .arg("--exact")
        .arg(test_name)
        .arg("--nocapture")
        .output()?;

    // Guard against a child that failed for an unrelated reason (bad args,
    // harness rejection, panic): anything other than a clean compile-failure
    // must not be reported as "the fixture was rejected".
    let stderr = String::from_utf8_lossy(&status.stderr);
    assert!(
        stderr.contains("restricted visibility")
            || stderr.contains("E04001")
            || stderr.contains("E06001")
            || stderr.contains("Unbound"),
        "child for fixture {package} did not report a visibility rejection; \
         the negative test is not exercising what it claims. stderr:\n{stderr}"
    );
    assert!(
        !status.status.success(),
        "{failure_message} (fixture: {package})"
    );
    Ok(())
}

/// True when this process was re-executed as a compile child.
fn is_child(sentinel: &str) -> bool {
    std::env::args().any(|a| a == sentinel)
}
