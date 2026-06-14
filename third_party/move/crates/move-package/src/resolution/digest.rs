// Copyright (c) The Diem Core Contributors
// Copyright (c) The Move Contributors
// SPDX-License-Identifier: Apache-2.0

use anyhow::Result;
use move_command_line_common::files::MOVE_EXTENSION;
use sha2::{Digest, Sha256};
use std::{
    borrow::Cow,
    path::{Path, PathBuf},
};

use crate::source_package::{layout::SourcePackageLayout, parsed_manifest::PackageDigest};

pub fn compute_digest(paths: &[PathBuf]) -> Result<PackageDigest> {
    let mut hashed_files = Vec::new();
    let mut hash = |path: &Path| {
        let contents = std::fs::read(path)?;
        hashed_files.push(digest_str(&normalize_source_bytes(&contents)));
        Ok(())
    };
    let mut maybe_hash_file = |path: &Path| -> Result<()> {
        match path.extension() {
            Some(x) if MOVE_EXTENSION == x => hash(path),
            _ if path.ends_with(SourcePackageLayout::Manifest.path()) => hash(path),
            _ => Ok(()),
        }
    };

    for path in paths {
        if path.is_file() {
            maybe_hash_file(path)?;
        } else {
            for entry in walkdir::WalkDir::new(path)
                .follow_links(true)
                .into_iter()
                .filter_map(|e| e.ok())
            {
                if entry.file_type().is_file() {
                    maybe_hash_file(entry.path())?
                }
            }
        }
    }

    Ok(PackageDigest::from(hashed_files_digest(hashed_files)))
}

pub fn hashed_files_digest(mut hashed_files: Vec<String>) -> String {
    // Sort the hashed files to ensure that the order of files is always stable
    hashed_files.sort();

    let mut hasher = Sha256::new();
    for file_hash in hashed_files.into_iter() {
        hasher.update(file_hash.as_bytes());
    }

    hex::encode_upper(hasher.finalize())
}

pub fn digest_str(data: &[u8]) -> String {
    hex::encode_upper(Sha256::digest(data))
}

fn normalize_source_bytes(contents: &[u8]) -> Cow<'_, [u8]> {
    if !contents.windows(2).any(|window| window == b"\r\n") {
        return Cow::Borrowed(contents);
    }

    let mut normalized = Vec::with_capacity(contents.len());
    let mut offset = 0;
    while offset < contents.len() {
        if contents[offset..].starts_with(b"\r\n") {
            normalized.push(b'\n');
            offset += 2;
        } else {
            normalized.push(contents[offset]);
            offset += 1;
        }
    }
    Cow::Owned(normalized)
}

#[cfg(test)]
mod tests {
    use super::{digest_str, normalize_source_bytes};

    #[test]
    fn digest_is_uppercase_sha256() {
        assert_eq!(
            digest_str(b""),
            "E3B0C44298FC1C149AFBF4C8996FB92427AE41E4649B934CA495991B7852B855"
        );
    }

    #[test]
    fn normalizes_crlf_without_changing_other_bytes() {
        assert_eq!(normalize_source_bytes(b"a\r\nb\r\n").as_ref(), b"a\nb\n");
        assert_eq!(normalize_source_bytes(b"a\rb\n").as_ref(), b"a\rb\n");
    }
}
