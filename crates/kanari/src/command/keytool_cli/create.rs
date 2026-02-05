// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

// Create a new wallet with kanari-crypto
//
// This command generates a new wallet with a secure private key and a seed phrase.
// The wallet is encrypted with the provided password and saved to disk.
use anyhow::{Context, Result};
use clap::Parser;
use kanari_crypto::keys::{CurveType, generate_keypair, generate_mnemonic, keypair_from_mnemonic};
use kanari_crypto::wallet::save_wallet;
use move_core_types::account_address::AccountAddress;
use std::str::FromStr;

#[derive(Parser, Debug)]
pub struct CreateWallet {
    /// Password for wallet encryption
    #[arg(short, long)]
    pub password: String,
    /// Curve type (ed25519, k256, p256, dilithium2, dilithium3, dilithium5, sphincs+, ed25519+dilithium3, k256+dilithium3)
    #[arg(short, long, default_value = "ed25519")]
    pub curve: String,
    /// Number of seed words (12 or 24)
    #[arg(short, long, default_value = "12")]
    pub words: usize,
}

impl CreateWallet {
    pub fn execute(&self) -> Result<()> {
        let curve_type = match self.curve.to_lowercase().as_str() {
            "ed25519" => CurveType::Ed25519,
            "k256" | "secp256k1" => CurveType::K256,
            "p256" | "secp256r1" => CurveType::P256,
            "dilithium2" => CurveType::Dilithium2,
            "dilithium3" => CurveType::Dilithium3,
            "dilithium5" => CurveType::Dilithium5,
            "sphincs+" | "sphincsplus" => CurveType::SphincsPlusSha256Robust,
            "ed25519+dilithium3" | "ed25519_dilithium3" => CurveType::Ed25519Dilithium3,
            "k256+dilithium3" | "k256_dilithium3" => CurveType::K256Dilithium3,
            other => {
                eprintln!("Unknown curve '{}', falling back to Ed25519", other);
                CurveType::Ed25519
            }
        };

        // For classical curves we can derive from a mnemonic; for PQC/hybrid generate directly
        let (private_key, address_str, seed_phrase) = if curve_type.is_post_quantum()
            || curve_type.is_hybrid()
        {
            let kp = generate_keypair(curve_type).context("Failed to generate keypair")?;
            let zk = kp.export_private_key_secure();
            (zk.to_string(), kp.get_address().to_string(), String::new())
        } else {
            let mnemonic = generate_mnemonic(self.words).context("Failed to generate mnemonic")?;
            let kp = keypair_from_mnemonic(&mnemonic, curve_type, "")
                .context("Failed to derive keypair from mnemonic")?;
            let zk = kp.export_private_key_secure();
            (zk.to_string(), kp.get_address().to_string(), mnemonic)
        };

        let address =
            AccountAddress::from_str(&address_str).context("Generated invalid address")?;

        // Save wallet
        save_wallet(
            &address,
            &private_key,
            &seed_phrase,
            None,
            &self.password,
            curve_type,
        )
        .context("Failed to save wallet")?;

        eprintln!("Created wallet: {}", address_str);
        if !seed_phrase.is_empty() {
            eprintln!("Seed phrase: {}", seed_phrase);
        }

        Ok(())
    }
}
