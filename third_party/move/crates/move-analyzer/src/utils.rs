// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::{Result, anyhow};
use codespan_reporting::files::{Files, SimpleFiles};
use lsp_types::{Position, Uri};
use move_command_line_common::files::FileHash;
use move_ir_types::location::*;
use move_symbol_pool::Symbol;
use std::{
    collections::HashMap,
    path::{Path, PathBuf},
};
use url::Url;

pub fn uri_to_file_path(uri: &Uri) -> Result<PathBuf> {
    Url::parse(uri.as_str())?
        .to_file_path()
        .map_err(|_| anyhow!("URI is not a valid file URI: {}", uri.as_str()))
}

pub fn path_to_uri(path: &Path) -> Result<Uri> {
    Url::from_file_path(path)
        .map_err(|_| {
            anyhow!(
                "path cannot be represented as a file URI: {}",
                path.display()
            )
        })?
        .as_str()
        .parse()
        .map_err(|err| anyhow!("failed to parse file URI for {}: {err}", path.display()))
}

/// Converts a location from the byte index format to the line/character (Position) format, where
/// line/character are 0-based.
pub fn get_loc(
    fhash: &FileHash,
    pos: ByteIndex,
    files: &SimpleFiles<Symbol, String>,
    file_id_mapping: &HashMap<FileHash, usize>,
) -> Option<Position> {
    let id = file_id_mapping.get(fhash)?;
    match files.location(*id, pos as usize) {
        Ok(v) => Some(Position {
            // we need 0-based column location
            line: v.line_number as u32 - 1,
            character: v.column_number as u32 - 1,
        }),
        Err(_) => None,
    }
}
