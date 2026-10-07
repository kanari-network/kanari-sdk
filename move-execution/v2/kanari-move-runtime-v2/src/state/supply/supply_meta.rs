// Copyright (c) KanariNetwork, Inc.
// SPDX-License-Identifier: Apache-2.0

//! Token metadata fields (decimals, name, symbol, description, icon).

use super::*;

impl StateManager {
    fn save_token_metadata_field<T: Serialize + ?Sized>(
        &mut self,
        prefix: &[u8],
        token_type: &str,
        value: &T,
    ) -> Result<()> {
        // Always key metadata by the canonical token type so lookups never
        // miss due to `0x02` vs `0x2` style spellings.
        let normalized = Self::normalize_token_type(token_type);
        let key = metadata_key(prefix, &normalized);
        self.save_internal(&key, value)
    }

    fn load_token_metadata_field<T: DeserializeOwned>(
        &self,
        prefix: &[u8],
        token_type: &str,
    ) -> Result<Option<T>> {
        // Try canonical key first, then the raw spelling for DBs written
        // before normalization was enforced. A corrupt entry (key holds
        // another type's bytes) is treated as unknown, never fatal: metadata
        // is best-effort display data, and every caller already degrades to
        // None. Storage errors still propagate via save paths.
        let normalized = Self::normalize_token_type(token_type);
        let key = metadata_key(prefix, &normalized);
        match self.load_internal::<T>(&key) {
            Ok(Some(value)) => return Ok(Some(value)),
            Ok(None) => {}
            Err(_) => return Ok(None),
        }
        if normalized != token_type {
            let raw_key = metadata_key(prefix, token_type);
            match self.load_internal::<T>(&raw_key) {
                Ok(value) => return Ok(value),
                Err(_) => return Ok(None),
            }
        }
        Ok(None)
    }
    /// Persist CoinMetadata fields for the given token type.
    pub(crate) fn persist_coin_metadata(&mut self, token_type: &str, data: &[u8]) -> Result<()> {
        #[derive(Deserialize)]
        struct MoveString {
            bytes: Vec<u8>,
        }
        #[derive(Deserialize)]
        struct MoveUrl {
            inner: MoveString,
        }
        #[derive(Deserialize)]
        struct MoveOption<T> {
            vec: Vec<T>,
        }
        #[derive(Deserialize)]
        struct ParsedCoinMetadata {
            id: AccountAddress,
            decimals: u8,
            // On-chain order is `name` then `symbol`
            // (see kanari_system::coin::CoinMetadata).
            name: MoveString,
            symbol: MoveString,
            description: MoveString,
            icon_url: MoveOption<MoveUrl>,
        }

        if let Ok(meta) = bcs::from_bytes::<ParsedCoinMetadata>(data) {
            // Required for canonical BCS layout; metadata is keyed by token type.
            let _ = meta.id;
            self.save_token_metadata_field(b"metadata_decimals:", token_type, &meta.decimals)?;

            if let Ok(name) = String::from_utf8(meta.name.bytes) {
                self.save_token_metadata_field(b"metadata_name:", token_type, &name)?;
            }
            if let Ok(symbol) = String::from_utf8(meta.symbol.bytes) {
                self.save_token_metadata_field(b"metadata_symbol:", token_type, &symbol)?;
            }
            if let Ok(description) = String::from_utf8(meta.description.bytes) {
                self.save_token_metadata_field(b"metadata_description:", token_type, &description)?;
            }
            if let Some(url_obj) = meta.icon_url.vec.into_iter().next()
                && let Ok(url) = String::from_utf8(url_obj.inner.bytes)
            {
                self.save_token_metadata_field(b"metadata_icon_url:", token_type, &url)?;
            }
        } else if data.len() > 32 {
            // Validate: Move caps decimals at 9; never persist a corrupt byte.
            let decimals = data[32];
            if decimals <= 9 {
                self.save_token_metadata_field(b"metadata_decimals:", token_type, &decimals)?;
            }
        }
        Ok(())
    }

    /// Get token decimals for a specific token type
    pub fn get_token_decimals(&self, token_type: &str) -> Result<Option<u8>> {
        self.load_token_metadata_field(b"metadata_decimals:", token_type)
    }

    ///  Get token name for a specific token type
    pub fn get_token_name(&self, token_type: &str) -> Result<Option<String>> {
        self.load_token_metadata_field(b"metadata_name:", token_type)
    }

    ///  Get token symbol for a specific token type
    pub fn get_token_symbol(&self, token_type: &str) -> Result<Option<String>> {
        self.load_token_metadata_field(b"metadata_symbol:", token_type)
    }

    /// Get token description for a specific token type
    pub fn get_token_description(&self, token_type: &str) -> Result<Option<String>> {
        self.load_token_metadata_field(b"metadata_description:", token_type)
    }

    /// Get token icon URL for a specific token type
    pub fn get_token_icon_url(&self, token_type: &str) -> Result<Option<String>> {
        self.load_token_metadata_field(b"metadata_icon_url:", token_type)
    }
}
