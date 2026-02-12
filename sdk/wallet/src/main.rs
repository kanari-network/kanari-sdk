// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

use kanari_crypto::hd_wallet::{HdError, derive_multiple_addresses};
use kanari_crypto::keys::CurveType;

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

    eprintln!("--- HD Wallet Generation System ---");
    eprintln!("Seed Phrase: {}", mnemonic);
    eprintln!("Derivation Path Template: {}", path_template);
    eprintln!("Curve Type: {:?}", curve);
    eprintln!("--------------------------------------------------");

    // Generate multiple KeyPairs from the template
    let keypairs = derive_multiple_addresses(mnemonic, password, path_template, curve, count)?;

    for (i, kp) in keypairs.iter().enumerate() {
        // Securely export the Private Key (data will be zeroized after use)
        let private_key = kp.export_private_key_secure();

        eprintln!("Wallet #{}:", i + 1);
        eprintln!("  1. Address: {}", kp.address);
        eprintln!("  2. Private Key (PK): {}", *private_key);
        eprintln!("--------------------------------------------------");
    }

    Ok(())
}
