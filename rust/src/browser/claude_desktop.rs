//! Claude Desktop package directories used by Agent Sessions.

use std::path::{Path, PathBuf};

const APP_DIR: &str = "Claude";
const MSIX_PACKAGE_PREFIX: &str = "claude_";

/// Microsoft Store (MSIX) Claude package stores under `local`, sorted. The
/// directories are not checked for existence.
pub(crate) fn msix_store_dirs(local: &Path) -> Vec<PathBuf> {
    let Ok(entries) = std::fs::read_dir(local.join("Packages")) else {
        return Vec::new();
    };
    let mut dirs: Vec<PathBuf> = entries
        .flatten()
        .filter(|entry| {
            entry
                .file_name()
                .to_str()
                .is_some_and(|name| name.to_ascii_lowercase().starts_with(MSIX_PACKAGE_PREFIX))
        })
        .map(|entry| {
            entry
                .path()
                .join("LocalCache")
                .join("Roaming")
                .join(APP_DIR)
        })
        .collect();
    dirs.sort();
    dirs
}
