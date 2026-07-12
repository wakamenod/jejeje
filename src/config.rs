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
