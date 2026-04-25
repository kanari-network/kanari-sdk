// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Email validation utilities
//!
//! This module provides comprehensive email validation to ensure
//! only properly formatted email addresses are accepted.

use regex::Regex;
use crate::{AuthError, AuthResult};

/// Validates an email address format
///
/// # Arguments
/// * `email` - The email address to validate
///
/// # Returns
/// * `Ok(())` if the email is valid
/// * `Err(AuthError)` if the email is invalid
///
/// # Examples
/// ```
/// use kanari_auth::email_validator::validate_email;
///
/// assert!(validate_email("user@example.com").is_ok());
/// assert!(validate_email("invalid-email").is_err());
/// ```
pub fn validate_email(email: &str) -> AuthResult<()> {
    // Check basic length constraints
    if email.is_empty() {
        return Err(AuthError::InvalidEmail("Email cannot be empty".to_string()));
    }

    if email.len() > 254 {
        return Err(AuthError::InvalidEmail(
            "Email too long (max 254 characters)".to_string(),
        ));
    }

    // RFC 5322 compliant email regex
    let email_regex = Regex::new(
        r"^[a-zA-Z0-9.!#$%&'*+/=?^_`{|}~-]+@[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?(?:\.[a-zA-Z0-9](?:[a-zA-Z0-9-]{0,61}[a-zA-Z0-9])?)*$",
    ).map_err(|e| AuthError::ValidationError(format!("Failed to compile email regex: {}", e)))?;

    if !email_regex.is_match(email) {
        return Err(AuthError::InvalidEmail(format!(
            "Invalid email format: {}",
            email
        )));
    }

    // Additional checks
    let parts: Vec<&str> = email.split('@').collect();
    if parts.len() != 2 {
        return Err(AuthError::InvalidEmail(
            "Email must contain exactly one @ symbol".to_string(),
        ));
    }

    let local_part = parts[0];
    let domain_part = parts[1];

    // Validate local part (before @)
    if local_part.is_empty() {
        return Err(AuthError::InvalidEmail(
            "Local part (before @) cannot be empty".to_string(),
        ));
    }

    if local_part.len() > 64 {
        return Err(AuthError::InvalidEmail(
            "Local part too long (max 64 characters)".to_string(),
        ));
    }

    // Validate domain part (after @)
    if domain_part.is_empty() {
        return Err(AuthError::InvalidEmail(
            "Domain part (after @) cannot be empty".to_string(),
        ));
    }

    if !domain_part.contains('.') {
        return Err(AuthError::InvalidEmail(
            "Domain must contain at least one dot".to_string(),
        ));
    }

    // Check for consecutive dots
    if email.contains("..") {
        return Err(AuthError::InvalidEmail(
            "Email cannot contain consecutive dots".to_string(),
        ));
    }

    Ok(())
}

/// Normalizes an email address by converting to lowercase and trimming whitespace
///
/// # Arguments
/// * `email` - The email address to normalize
///
/// # Returns
/// Normalized email address
pub fn normalize_email(email: &str) -> String {
    email.trim().to_lowercase()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_valid_emails() {
        let valid_emails = vec![
            "user@example.com",
            "john.doe@company.org",
            "admin+test@sub.domain.co.uk",
            "user123@test.io",
            "first_last@name.com",
            "user.name+tag@example.com",
        ];

        for email in valid_emails {
            assert!(
                validate_email(email).is_ok(),
                "Expected {} to be valid",
                email
            );
        }
    }

    #[test]
    fn test_invalid_emails() {
        let invalid_emails = vec![
            "",
            "not-an-email",
            "@example.com",
            "user@",
            "user@.com",
            "user@@example.com",
            "user..name@example.com",
            "user@exam ple.com",
            "user@example",
        ];

        for email in invalid_emails {
            assert!(
                validate_email(email).is_err(),
                "Expected {} to be invalid",
                email
            );
        }
    }

    #[test]
    fn test_normalize_email() {
        assert_eq!(normalize_email("User@Example.COM"), "user@example.com");
        assert_eq!(normalize_email("  Test@Test.com  "), "test@test.com");
        assert_eq!(normalize_email("ADMIN@DOMAIN.ORG"), "admin@domain.org");
    }

    #[test]
    fn test_email_length_limits() {
        // Local part too long (> 64 chars)
        let long_local = format!("{}@example.com", "a".repeat(65));
        assert!(validate_email(&long_local).is_err());

        // Total email too long (> 254 chars)
        let long_email = format!("{}@{}.com", "a".repeat(200), "b".repeat(50));
        assert!(validate_email(&long_email).is_err());
    }
}
