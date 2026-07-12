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
    /// Directory name for the contest (default: uses contest_id as-is)
    #[serde(default = "default_contest_directory")]
    pub contest_directory: String,

    /// Directory name for each task (default: uses task_id as-is)
    #[serde(default = "default_task_directory")]
    pub task_directory: String,

    /// Sub-directory name for sample test cases (default: "test")
    #[serde(default = "default_test_directory")]
    pub test_directory: String,

    /// Default template name applied when none is specified on the command line
    #[serde(default)]
    pub default_template: Option<String>,

    /// Path to the directory that contains template directories
    #[serde(default)]
    pub template_dir: Option<String>,
}

fn default_contest_directory() -> String {
    "{contest_id}".to_string()
}

fn default_task_directory() -> String {
    "{task_id}".to_string()
}

fn default_test_directory() -> String {
    "test".to_string()
}

impl Default for Config {
    fn default() -> Self {
        Self {
            contest_directory: default_contest_directory(),
            task_directory: default_task_directory(),
            test_directory: default_test_directory(),
            default_template: None,
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
    fn default_contest_directory() {
        let config = Config::default();
        assert_eq!(config.contest_directory, "{contest_id}");
    }

    #[test]
    fn default_task_directory() {
        let config = Config::default();
        assert_eq!(config.task_directory, "{task_id}");
    }

    #[test]
    fn default_test_directory() {
        let config = Config::default();
        assert_eq!(config.test_directory, "test");
    }

    #[test]
    fn default_optional_fields_are_none() {
        let config = Config::default();
        assert!(config.default_template.is_none());
        assert!(config.template_dir.is_none());
    }

    // ─── TOML シリアライズ / デシリアライズ ───────────────────────

    #[test]
    fn toml_roundtrip_defaults() {
        let original = Config::default();
        let toml_str = toml::to_string_pretty(&original).unwrap();
        let restored: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(original.contest_directory, restored.contest_directory);
        assert_eq!(original.task_directory, restored.task_directory);
        assert_eq!(original.test_directory, restored.test_directory);
        assert_eq!(original.default_template, restored.default_template);
        assert_eq!(original.template_dir, restored.template_dir);
    }

    #[test]
    fn toml_roundtrip_with_optional_fields() {
        let original = Config {
            contest_directory: "contest_{contest_id}".to_string(),
            task_directory: "task_{task_id}".to_string(),
            test_directory: "samples".to_string(),
            default_template: Some("rust".to_string()),
            template_dir: Some("/home/user/templates".to_string()),
        };
        let toml_str = toml::to_string_pretty(&original).unwrap();
        let restored: Config = toml::from_str(&toml_str).unwrap();
        assert_eq!(restored.contest_directory, "contest_{contest_id}");
        assert_eq!(restored.task_directory, "task_{task_id}");
        assert_eq!(restored.test_directory, "samples");
        assert_eq!(restored.default_template, Some("rust".to_string()));
        assert_eq!(restored.template_dir, Some("/home/user/templates".to_string()));
    }

    #[test]
    fn toml_partial_fields_get_defaults() {
        // TOML に一部フィールドのみ書いた場合、残りはデフォルトになる
        let toml_str = r#"test_directory = "cases""#;
        let config: Config = toml::from_str(toml_str).unwrap();
        assert_eq!(config.test_directory, "cases");
        assert_eq!(config.contest_directory, "{contest_id}");
        assert_eq!(config.task_directory, "{task_id}");
        assert!(config.default_template.is_none());
    }

    // ─── save() / load() ─────────────────────────────────────────

    #[test]
    fn save_and_load_via_tempdir() {
        use std::fs;
        let dir = tempfile::tempdir().unwrap();
        let path = dir.path().join("config.toml");

        let original = Config {
            contest_directory: "my_contest".to_string(),
            task_directory: "my_task".to_string(),
            test_directory: "t".to_string(),
            default_template: Some("cpp".to_string()),
            template_dir: None,
        };

        // 直接ファイルに書き込む（save() は OS config dir を使うため手動で実施）
        let toml_str = toml::to_string_pretty(&original).unwrap();
        fs::write(&path, &toml_str).unwrap();

        // 読み込んで検証
        let content = fs::read_to_string(&path).unwrap();
        let loaded: Config = toml::from_str(&content).unwrap();
        assert_eq!(loaded.contest_directory, "my_contest");
        assert_eq!(loaded.task_directory, "my_task");
        assert_eq!(loaded.test_directory, "t");
        assert_eq!(loaded.default_template, Some("cpp".to_string()));
        assert!(loaded.template_dir.is_none());
    }
}
