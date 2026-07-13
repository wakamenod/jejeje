use crate::error::AppError;
use directories::ProjectDirs;
use serde::{Deserialize, Serialize};
use std::path::PathBuf;

/// Global configuration stored at the OS config directory.
///
/// Linux:   ~/.config/je/config.toml
/// macOS:   ~/Library/Application Support/je/config.toml
/// Windows: %APPDATA%\je\config.toml
#[derive(Debug, Serialize, Deserialize)]
pub struct Config {
    /// Path to the directory whose files are copied into each task directory
    #[serde(default)]
    pub template_dir: Option<String>,
}

impl Default for Config {
    fn default() -> Self {
        Self {
            template_dir: None,
        }
    }
}

impl Config {
    /// Returns the path to `config.toml`.
    pub fn config_path() -> Result<PathBuf, AppError> {
        let dirs = ProjectDirs::from("", "", "je").ok_or(AppError::ConfigDirNotFound)?;
        Ok(dirs.config_dir().join("config.toml"))
    }

    /// Load config, returning defaults when the file does not yet exist.
    pub fn load() -> Result<Self, AppError> {
        let path = Self::config_path()?;
        if !path.exists() {
            return Ok(Self::default());
        }
        let content = std::fs::read_to_string(&path)?;
        Ok(toml::from_str(&content)?)
    }

    /// Persist the current config to disk, creating the directory if necessary.
    pub fn save(&self) -> anyhow::Result<()> {
        let path = Self::config_path()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let content = toml::to_string_pretty(self)?;
        std::fs::write(&path, content)?;
        Ok(())
    }

    /// Return the config path as a display string (for `je config` output).
    pub fn config_path_display() -> String {
        Self::config_path()
            .map(|p| p.display().to_string())
            .unwrap_or_else(|_| "<unknown>".to_string())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── デフォルト値 ─────────────────────────────────────────────

    #[test]
    fn default_optional_fields_are_none() {
        let config = Config::default();
        assert!(config.template_dir.is_none());
    }

    // ─── TOML シリアライズ / デシリアライズ ───────────────────────

    #[test]
    fn toml_roundtrip_defaults() {
        let original = Config::default();
        let toml_str = toml::to_string_pretty(&original).unwrap();
        let restored: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(original.template_dir, restored.template_dir);
    }

    #[test]
    fn toml_roundtrip_with_optional_fields() {
        let original = Config {
            template_dir: Some("/home/user/templates".to_string()),
        };
        let toml_str = toml::to_string_pretty(&original).unwrap();
        let restored: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(restored.template_dir, Some("/home/user/templates".to_string()));
    }

    // ─── save() / load() ─────────────────────────────────────────

    #[test]
    fn save_and_load_via_tempdir() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let original = Config {
            template_dir: Some("/home/user/templates".to_string()),
        };

        // 直接ファイルに書き込む（save() は OS config dir を使うため手動で実施）
        let toml_str = toml::to_string_pretty(&original).unwrap();
        fs::write(&path, &toml_str).unwrap();

        // 読み込んで検証
        let content = fs::read_to_string(&path).unwrap();
        let loaded: Config = toml::from_str(&content).unwrap();
        assert_eq!(loaded.template_dir, Some("/home/user/templates".to_string()));
    }
}
