use crate::{error::AppError, judge::model::ContestMeta};
use anyhow::{Context, Result};
use std::path::{Path, PathBuf};

/// Filename of the metadata file placed in the contest root directory.
pub const META_FILENAME: &str = ".je-meta.json";

/// Persist contest metadata to `<dir>/.je-meta.json`.
pub fn save(dir: &Path, meta: &ContestMeta) -> Result<()> {
    let path = dir.join(META_FILENAME);
    let json = serde_json::to_string_pretty(meta)?;
    std::fs::write(&path, json)
        .with_context(|| format!("Failed to write metadata to {}", path.display()))?;
    Ok(())
}

/// Walk up the directory tree from `start` and load the first `.je-meta.json` found.
///
/// Returns an error if no metadata file exists in any ancestor directory.
pub fn load(start: &Path) -> Result<ContestMeta> {
    let path = find(start).ok_or(AppError::MetaNotFound)?;
    let json = std::fs::read_to_string(&path)
        .with_context(|| format!("Failed to read {}", path.display()))?;
    let meta: ContestMeta = serde_json::from_str(&json)?;
    Ok(meta)
}

/// Returns the path of the `.je-meta.json` file found by walking up from `start`,
/// or `None` if not found.
pub fn find(start: &Path) -> Option<PathBuf> {
    let mut current = start.to_path_buf();
    loop {
        let candidate = current.join(META_FILENAME);
        if candidate.exists() {
            return Some(candidate);
        }
        match current.parent() {
            Some(parent) => current = parent.to_path_buf(),
            None => return None,
        }
    }
}

/// Returns the contest root directory (the directory that contains `.je-meta.json`).
pub fn find_contest_root(start: &Path) -> Option<PathBuf> {
    find(start).and_then(|p| p.parent().map(PathBuf::from))
}
