//! This module defines canonical error codes adopted from Google's canonical error codes.
//! Each code has an associated HTTP error code for REST APIs.

/// Error categories for canonical error codes
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ErrorCategory;

impl ErrorCategory {
    // Error category constants
    pub const INVALID_ARGUMENT: u64 = 0x1;      // http: 400
    pub const OUT_OF_RANGE: u64 = 0x2;          // http: 400
    pub const INVALID_STATE: u64 = 0x3;         // http: 400
    pub const UNAUTHENTICATED: u64 = 0x4;       // http: 401
    pub const PERMISSION_DENIED: u64 = 0x5;      // http: 403
    pub const NOT_FOUND: u64 = 0x6;             // http: 404
    pub const ABORTED: u64 = 0x7;               // http: 409
    pub const ALREADY_EXISTS: u64 = 0x8;        // http: 409
    pub const RESOURCE_EXHAUSTED: u64 = 0x9;    // http: 429
    pub const CANCELLED: u64 = 0xA;             // http: 499
    pub const INTERNAL: u64 = 0xB;              // http: 500
    pub const NOT_IMPLEMENTED: u64 = 0xC;       // http: 501
    pub const UNAVAILABLE: u64 = 0xD;           // http: 503

    /// Construct a canonical error code from a category and reason
    pub fn canonical(category: u64, reason: u64) -> u64 {
        (category << 16) + reason
    }

    // Helper functions to construct canonical error codes
    pub fn invalid_argument(reason: u64) -> u64 { Self::canonical(Self::INVALID_ARGUMENT, reason) }
    pub fn out_of_range(reason: u64) -> u64 { Self::canonical(Self::OUT_OF_RANGE, reason) }
    pub fn invalid_state(reason: u64) -> u64 { Self::canonical(Self::INVALID_STATE, reason) }
    pub fn unauthenticated(reason: u64) -> u64 { Self::canonical(Self::UNAUTHENTICATED, reason) }
    pub fn permission_denied(reason: u64) -> u64 { Self::canonical(Self::PERMISSION_DENIED, reason) }
    pub fn not_found(reason: u64) -> u64 { Self::canonical(Self::NOT_FOUND, reason) }
    pub fn aborted(reason: u64) -> u64 { Self::canonical(Self::ABORTED, reason) }
    pub fn already_exists(reason: u64) -> u64 { Self::canonical(Self::ALREADY_EXISTS, reason) }
    pub fn resource_exhausted(reason: u64) -> u64 { Self::canonical(Self::RESOURCE_EXHAUSTED, reason) }
    pub fn internal(reason: u64) -> u64 { Self::canonical(Self::INTERNAL, reason) }
    pub fn not_implemented(reason: u64) -> u64 { Self::canonical(Self::NOT_IMPLEMENTED, reason) }
    pub fn unavailable(reason: u64) -> u64 { Self::canonical(Self::UNAVAILABLE, reason) }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_canonical_error_codes() {
        let reason = 0x3;
        assert_eq!(
            ErrorCategory::invalid_argument(reason),
            0x10003
        );
        assert_eq!(
            ErrorCategory::not_found(reason),
            0x60003
        );
    }

    #[test]
    fn test_error_categories() {
        assert_eq!(ErrorCategory::INVALID_ARGUMENT, 0x1);
        assert_eq!(ErrorCategory::NOT_FOUND, 0x6);
        assert_eq!(ErrorCategory::INTERNAL, 0xB);
    }
}