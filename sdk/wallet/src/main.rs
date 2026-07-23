// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_crypto::hd_wallet::{HdError, derive_multiple_addresses};
use kanari_crypto::keys::CurveType;
use std::env;

fn main() -> Result<(), HdError> {
    // Example seed phrase (Mnemonic) - In production, this should be read from input or environment
    let mnemonic = "abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon abandon about";
    let password = ""; // Optional password for the seed

    // BIP44 derivation path template: m/44'/637'/0'/0/{index}
    // 637 is the assumed coin type for Kanari
    let path_template = "m/44'/637'/0'/0/{index}";

    // Using P256 (secp256r1) curve
    let curve = CurveType::P256;

    // Number of wallets to generate
    let count = 10;
    let show_secrets = env::var("KANARI_WALLET_EXAMPLE_SHOW_SECRETS")
        .map(|value| value == "1" || value.eq_ignore_ascii_case("true"))
        .unwrap_or(false);

    eprintln!("--- HD Wallet Generation System ---");
    if show_secrets {
        eprintln!("Seed Phrase: {}", mnemonic);
    } else {
        eprintln!("Seed Phrase: <redacted; set KANARI_WALLET_EXAMPLE_SHOW_SECRETS=1 to print>");
    }
    eprintln!("Derivation Path Template: {}", path_template);
    eprintln!("Curve Type: {:?}", curve);
    eprintln!("--------------------------------------------------");

    // Generate multiple KeyPairs from the template
    let keypairs = derive_multiple_addresses(mnemonic, password, path_template, curve, count)?;

    for (i, kp) in keypairs.iter().enumerate() {
        eprintln!("Wallet #{}:", i + 1);
        eprintln!("  1. Address: {}", kp.address);
        if show_secrets {
            // Securely export the Private Key (data will be zeroized after use)
            let private_key = kp.export_private_key_secure();
            eprintln!("  2. Private Key (PK): {}", *private_key);
        } else {
            eprintln!("  2. Private Key (PK): <redacted>");
        }
        eprintln!("--------------------------------------------------");
    }

    Ok(())
}
