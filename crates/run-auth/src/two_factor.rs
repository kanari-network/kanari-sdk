// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Two-Factor Authentication (2FA) support using TOTP
//!
//! This module provides Time-based One-Time Password (TOTP) functionality
//! for enhanced security on user accounts.

use serde::{Deserialize, Serialize};
use totp_rs::{Algorithm, TOTP};

/// 2FA setup information returned after enabling 2FA
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TwoFactorSetup {
    /// Secret key (base32 encoded) - show as QR code
    pub secret: String,

    /// OTPAuth URL for QR code generation
    pub otpauth_url: String,

    /// Backup codes for account recovery (one-time use)
    pub backup_codes: Vec<String>,
}

/// TOTP verification request
#[derive(Debug, Deserialize)]
pub struct TotpVerifyRequest {
    /// User's email
    pub email: String,

    /// 6-digit TOTP code
    pub code: String,
}

/// Enable 2FA request
#[derive(Debug, Deserialize)]
pub struct Enable2faRequest {
    /// User's email
    pub email: String,

    /// User's password for verification
    pub password: String,

    /// TOTP code to verify setup
    pub code: String,
}

/// Disable 2FA request
#[derive(Debug, Deserialize)]
pub struct Disable2faRequest {
    /// User's email
    pub email: String,

    /// User's password for verification
    pub password: String,
}

/// TOTP Manager for handling 2FA operations
#[derive(Clone)]
pub struct TotpManager {
    /// Issuer name for OTPAuth URL
    issuer: String,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub enum VerificationMethod {
    Totp,
    BackupCode,
}

impl TotpManager {
    /// Create a new TOTP manager
    pub fn new(issuer: Option<String>) -> Self {
        Self {
            issuer: issuer.unwrap_or_else(|| "Kanari Auth".to_string()),
        }
    }

    /// Generate a new TOTP secret and setup information
    pub fn generate_setup(&self, email: &str) -> TwoFactorSetup {
        // Generate random secret bytes directly
        let secret_bytes: Vec<u8> = (0..20).map(|_| rand::random::<u8>()).collect();

        // Create TOTP instance with raw bytes
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,  // 6 digits
            1,  // 1 step skew
            30, // 30 second interval
            secret_bytes.clone(),
            Some(self.issuer.clone()),
            email.to_string(),
        )
        .expect("Failed to create TOTP");

        // Generate backup codes (10 codes)
        let backup_codes = (0..10)
            .map(|_| {
                // Generate random 8-character alphanumeric code
                let code: String = (0..8)
                    .map(|_| {
                        let chars = b"ABCDEFGHIJKLMNOPQRSTUVWXYZ0123456789";
                        let idx = rand::random::<usize>() % chars.len();
                        chars[idx] as char
                    })
                    .collect();
                code
            })
            .collect();

        // Get OTPAuth URL
        let otpauth_url = totp.get_url();

        // Encode secret as base32 for display
        let secret_b32 = data_encoding::BASE32_NOPAD.encode(&secret_bytes);

        TwoFactorSetup {
            secret: secret_b32,
            otpauth_url,
            backup_codes,
        }
    }

    /// Verify a TOTP code against a secret
    pub fn verify_code(&self, secret: &str, code: &str) -> bool {
        // Parse the base32 secret back to bytes
        let secret_bytes = match data_encoding::BASE32_NOPAD.decode(secret.as_bytes()) {
            Ok(bytes) => bytes,
            Err(_) => return false,
        };

        match TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_bytes,
            Some(self.issuer.clone()),
            String::new(),
        ) {
            Ok(totp) => totp.check_current(code).unwrap_or(false),
            Err(_) => false,
        }
    }

    /// Generate QR code SVG for the OTPAuth URL
    pub fn generate_qr_svg(&self, otpauth_url: &str) -> Result<String, Box<dyn std::error::Error>> {
        use qrcode::QrCode;

        let code = QrCode::new(otpauth_url)?;
        let svg = code
            .render::<qrcode::render::svg::Color>()
            .min_dimensions(200, 200)
            .dark_color(qrcode::render::svg::Color("#000000"))
            .light_color(qrcode::render::svg::Color("#ffffff"))
            .build();

        Ok(svg)
    }

    /// Consume a one-time backup code from a mutable list.
    pub fn consume_backup_code(
        &self,
        backup_codes: &mut Vec<String>,
        backup_code: &str,
    ) -> Result<VerificationMethod, &'static str> {
        if let Some(index) = backup_codes.iter().position(|stored| stored == backup_code) {
            backup_codes.remove(index);
            Ok(VerificationMethod::BackupCode)
        } else {
            Err("Invalid backup code")
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_totp_generation_and_verification() {
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let manager = TotpManager::new(Some("Test".to_string()));

        // Generate setup
        let setup = runtime.block_on(async { manager.generate_setup("test@example.com") });

        assert!(!setup.secret.is_empty());
        assert!(!setup.otpauth_url.is_empty());
        assert_eq!(setup.backup_codes.len(), 10);

        // Verify that the URL contains expected components
        assert!(setup.otpauth_url.contains("otpauth://totp/"));
        assert!(setup.otpauth_url.contains("Kanari%20Auth") || setup.otpauth_url.contains("Test"));
        assert!(setup.otpauth_url.contains("secret="));
    }

    #[test]
    fn test_backup_codes_format() {
        let manager = TotpManager::new(None);
        let runtime = tokio::runtime::Runtime::new().unwrap();
        let setup = runtime.block_on(async { manager.generate_setup("user@example.com") });

        for code in &setup.backup_codes {
            assert_eq!(code.len(), 8);
            assert!(code.chars().all(|c| c.is_alphanumeric()));
        }
    }

    #[test]
    fn test_enable_and_backup_code_verification() {
        let manager = TotpManager::new(Some("Test".to_string()));
        let setup = manager.generate_setup("test@example.com");

        let secret_bytes = data_encoding::BASE32_NOPAD
            .decode(setup.secret.as_bytes())
            .unwrap();
        let totp = TOTP::new(
            Algorithm::SHA1,
            6,
            1,
            30,
            secret_bytes,
            Some("Test".to_string()),
            "test@example.com".to_string(),
        )
        .unwrap();
        let code = totp.generate_current().unwrap();

        assert!(manager.verify_code(&setup.secret, &code));

        let mut backup_codes = setup.backup_codes.clone();
        let method = manager
            .consume_backup_code(&mut backup_codes, &setup.backup_codes[0])
            .unwrap();
        assert!(matches!(method, VerificationMethod::BackupCode));
        assert_eq!(backup_codes.len(), 9);
    }
}
