// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Shared helpers for reading tuning/diagnostic environment variables.
//!
//! Kanari treats environment flags as *local policy only*: they must never
//! change consensus results or deterministic state roots. Keeping their
//! parsing in one place makes that boundary explicit and avoids subtly
//! different truthy spellings across crates.

/// Whether `name` is set to an explicit truthy value.
///
/// Anything else, including an unset variable, is false.
#[must_use]
pub fn env_flag(name: &str) -> bool {
    std::env::var(name).as_deref().is_ok_and(flag_enabled)
}

/// Whether `name` is set to an explicit truthy value, accepting `on` and
/// trimming/case-folding the value.
///
/// Kept separate from [`env_flag`] because safety-guard overrides historically
/// accepted `on`; silent policy widening is not appropriate for other flags.
#[must_use]
pub fn env_flag_loose(name: &str) -> bool {
    std::env::var(name).as_deref().is_ok_and(flag_enabled_loose)
}

/// Whether `name` is set to exactly `expected`.
///
/// Used where an override is intentionally narrow (for example a compatibility
/// escape hatch that requires the literal value `1`).
#[must_use]
pub fn env_exact(name: &str, expected: &str) -> bool {
    std::env::var(name).ok().as_deref() == Some(expected)
}

/// Whether `name` is enabled by default and only disabled by an explicit
/// falsy value.
#[must_use]
pub fn env_flag_default_true(name: &str) -> bool {
    std::env::var(name).as_deref().map_or(true, flag_default_true)
}

/// Default-on interval override. An explicit `0` disables the interval;
/// unset/invalid values keep `default`.
#[must_use]
pub fn env_u64_interval(name: &str, default: u64) -> Option<u64> {
    match std::env::var(name) {
        Ok(value) if value.trim() == "0" => None,
        Ok(value) => Some(positive_u64(&value).unwrap_or(default)),
        Err(_) => Some(default),
    }
}

/// Positive `u64` override, falling back to `default` when unset/invalid.
#[must_use]
pub fn env_u64_positive(name: &str, default: u64) -> u64 {
    env_u64_positive_opt(name).unwrap_or(default)
}

/// Positive `u64` override without a default.
#[must_use]
pub fn env_u64_positive_opt(name: &str) -> Option<u64> {
    std::env::var(name).ok().as_deref().and_then(positive_u64)
}

/// Positive `usize` override, falling back to `default` when unset/invalid.
#[must_use]
pub fn env_usize_positive(name: &str, default: usize) -> usize {
    std::env::var(name).ok().as_deref().and_then(positive_usize).unwrap_or(default)
}

/// Positive `usize` override clamped to `min..=max`.
///
/// Unset and invalid values keep `default`; values outside the range are
/// clamped rather than discarded.
#[must_use]
pub fn env_usize_clamped(name: &str, default: usize, min: usize, max: usize) -> usize {
    std::env::var(name)
        .ok()
        .and_then(|value| value.parse::<usize>().ok())
        .map(|value| value.clamp(min, max))
        .unwrap_or(default)
}

fn flag_enabled(value: &str) -> bool {
    matches!(value, "1" | "true" | "TRUE" | "yes" | "YES")
}

/// Loose truthy parser used by safety-guard overrides.
#[must_use]
pub fn flag_enabled_loose(value: &str) -> bool {
    matches!(value.trim().to_ascii_lowercase().as_str(), "1" | "true" | "yes" | "on")
}

fn flag_default_true(value: &str) -> bool {
    !matches!(value.trim().to_ascii_lowercase().as_str(), "0" | "false" | "no" | "off")
}

fn positive_u64(value: &str) -> Option<u64> {
    value.trim().parse::<u64>().ok().filter(|value| *value > 0)
}

fn positive_usize(value: &str) -> Option<usize> {
    value.trim().parse::<usize>().ok().filter(|value| *value > 0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn truthy_and_falsy_spellings_are_stable() {
        for value in ["1", "true", "TRUE", "yes", "YES"] {
            assert!(flag_enabled(value), "expected {value} to be truthy");
        }
        for value in ["0", "false", "FALSE", "no", "NO", "", "maybe"] {
            assert!(!flag_enabled(value), "expected {value} to be falsy");
        }
    }

    #[test]
    fn loose_flags_preserve_guard_override_spellings() {
        for value in ["1", " true ", "ON", "yes"] {
            assert!(flag_enabled_loose(value), "expected {value} to be truthy");
        }
        for value in ["0", "off", "", "maybe"] {
            assert!(!flag_enabled_loose(value), "expected {value} to be falsy");
        }
    }

    #[test]
    fn default_true_only_honors_explicit_falsy_values() {
        for value in ["0", "false", "FALSE", "no", "NO", "off", " OFF "] {
            assert!(!flag_default_true(value), "expected {value} to disable the flag");
        }
        for value in ["1", "true", "yes", "", "maybe"] {
            assert!(flag_default_true(value), "expected {value} to keep the flag enabled");
        }
    }

    #[test]
    fn zero_disables_interval_overrides() {
        assert_eq!(positive_u64("0"), None);
        assert_eq!(env_u64_interval("KANARI_ENV_UNSET_INTERVAL", 64), Some(64));
    }

    #[test]
    fn positive_overrides_reject_zero_and_garbage() {
        assert_eq!(positive_u64("0"), None);
        assert_eq!(positive_u64("12"), Some(12));
        assert_eq!(positive_u64("garbage"), None);
        assert_eq!(positive_usize(" 7 "), Some(7));
        assert_eq!(positive_usize("-1"), None);
    }
}
