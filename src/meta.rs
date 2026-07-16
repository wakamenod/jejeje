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
        {
            let parent = current.parent()?;
            current = parent.to_path_buf()
        }
    }
}

/// Returns the contest root directory (the directory that contains `.je-meta.json`).
pub fn find_contest_root(start: &Path) -> Option<PathBuf> {
    find(start).and_then(|p| p.parent().map(PathBuf::from))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::judge::model::{ContestMeta, TaskMeta};
    use tempfile::tempdir;

    fn make_meta() -> ContestMeta {
        ContestMeta {
            judge: "atcoder".to_string(),
            contest_id: "abc001".to_string(),
            contest_name: "AtCoder Beginner Contest 001".to_string(),
            url: "https://atcoder.jp/contests/abc001".to_string(),
            tasks: vec![TaskMeta {
                id: "a".to_string(),
                name: "Two Sum".to_string(),
                url: "https://atcoder.jp/contests/abc001/tasks/abc001_a".to_string(),
            }],
        }
    }

    // ─── save / load ─────────────────────────────────────────────

    #[test]
    fn save_creates_meta_file() {
        let dir = tempdir().unwrap();
        let meta = make_meta();
        save(dir.path(), &meta).unwrap();
        assert!(dir.path().join(META_FILENAME).exists());
    }

    #[test]
    fn save_and_load_roundtrip() {
        let dir = tempdir().unwrap();
        let meta = make_meta();
        save(dir.path(), &meta).unwrap();
        let loaded = load(dir.path()).unwrap();
        assert_eq!(loaded.judge, meta.judge);
        assert_eq!(loaded.contest_id, meta.contest_id);
        assert_eq!(loaded.contest_name, meta.contest_name);
        assert_eq!(loaded.url, meta.url);
        assert_eq!(loaded.tasks.len(), 1);
        assert_eq!(loaded.tasks[0].id, "a");
    }

    #[test]
    fn load_returns_error_when_no_meta_file() {
        let dir = tempdir().unwrap();
        let result = load(dir.path());
        assert!(result.is_err());
    }

    // ─── find ────────────────────────────────────────────────────

    #[test]
    fn find_in_same_directory() {
        let dir = tempdir().unwrap();
        let meta = make_meta();
        save(dir.path(), &meta).unwrap();
        let found = find(dir.path()).unwrap();
        assert_eq!(found, dir.path().join(META_FILENAME));
    }

    #[test]
    fn find_in_parent_directory() {
        let dir = tempdir().unwrap();
        let meta = make_meta();
        save(dir.path(), &meta).unwrap();

        // サブディレクトリから探索
        let sub = dir.path().join("abc001").join("a");
        std::fs::create_dir_all(&sub).unwrap();
        let found = find(&sub).unwrap();
        assert_eq!(found, dir.path().join(META_FILENAME));
    }

    #[test]
    fn find_returns_none_when_no_meta_file() {
        let dir = tempdir().unwrap();
        // .je-meta.json が存在しない独立した一時ディレクトリ
        let result = find(dir.path());
        assert!(result.is_none());
    }

    // ─── find_contest_root ───────────────────────────────────────

    #[test]
    fn find_contest_root_returns_parent_of_meta() {
        let dir = tempdir().unwrap();
        let meta = make_meta();
        save(dir.path(), &meta).unwrap();

        let sub = dir.path().join("task_a");
        std::fs::create_dir_all(&sub).unwrap();
        let root = find_contest_root(&sub).unwrap();
        assert_eq!(root, dir.path());
    }

    #[test]
    fn find_contest_root_returns_none_when_no_meta_file() {
        let dir = tempdir().unwrap();
        assert!(find_contest_root(dir.path()).is_none());
    }
}
