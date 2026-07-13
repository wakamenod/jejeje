use crate::config::Config;
use anyhow::{bail, Result};

/// `je config [key] [value]` — 設定の表示・変更。
///
/// - 引数なし: 全設定を表示
/// - キーのみ: その設定値を表示
/// - キー＋値: 設定を更新
pub async fn run(key: Option<String>, value: Option<String>) -> Result<()> {
    let mut config = Config::load()?;

    match (key.as_deref(), value.as_deref()) {
        // 引数なし: 全設定を表示
        (None, _) => {
            println!("Config file: {}", Config::config_path_display());
            println!();
            println!("contest_directory  = {}", config.contest_directory);
            println!("task_directory     = {}", config.task_directory);
            println!("test_directory     = {}", config.test_directory);
            println!(
                "template_dir       = {}",
                config.template_dir.as_deref().unwrap_or("(none)")
            );
        }

        // キーのみ: その値を表示
        (Some(k), None) => {
            let v = get_value(&config, k)?;
            println!("{k} = {v}");
        }

        // キー＋値: 更新して保存
        (Some(k), Some(v)) => {
            set_value(&mut config, k, v)?;
            config.save()?;
            println!("Set {k} = {v}");
        }
    }

    Ok(())
}

fn get_value(config: &Config, key: &str) -> Result<String> {
    match key {
        "contest_directory" => Ok(config.contest_directory.clone()),
        "task_directory" => Ok(config.task_directory.clone()),
        "test_directory" => Ok(config.test_directory.clone()),
        "template_dir" => Ok(config
            .template_dir
            .clone()
            .unwrap_or_else(|| "(none)".to_string())),
        _ => bail!("Unknown config key: '{key}'"),
    }
}

fn set_value(config: &mut Config, key: &str, value: &str) -> Result<()> {
    match key {
        "contest_directory" => config.contest_directory = value.to_string(),
        "task_directory" => config.task_directory = value.to_string(),
        "test_directory" => config.test_directory = value.to_string(),
        "template_dir" => {
            config.template_dir = if value == "(none)" || value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        }
        _ => bail!("Unknown config key: '{key}'"),
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::config::Config;

    fn default_config() -> Config {
        Config::default()
    }

    // ─── get_value ───────────────────────────────────────────────

    #[test]
    fn get_contest_directory() {
        let config = default_config();
        assert_eq!(get_value(&config, "contest_directory").unwrap(), "{contest_id}");
    }

    #[test]
    fn get_task_directory() {
        let config = default_config();
        assert_eq!(get_value(&config, "task_directory").unwrap(), "{task_id}");
    }

    #[test]
    fn get_test_directory() {
        let config = default_config();
        assert_eq!(get_value(&config, "test_directory").unwrap(), "test");
    }

    #[test]
    fn get_template_dir_none() {
        let config = default_config();
        assert_eq!(get_value(&config, "template_dir").unwrap(), "(none)");
    }

    #[test]
    fn get_template_dir_some() {
        let mut config = default_config();
        config.template_dir = Some("/path/to/templates".to_string());
        assert_eq!(
            get_value(&config, "template_dir").unwrap(),
            "/path/to/templates"
        );
    }

    #[test]
    fn get_unknown_key_returns_error() {
        let config = default_config();
        assert!(get_value(&config, "unknown_key").is_err());
    }

    // ─── set_value ───────────────────────────────────────────────

    #[test]
    fn set_contest_directory() {
        let mut config = default_config();
        set_value(&mut config, "contest_directory", "contests/{contest_id}").unwrap();
        assert_eq!(config.contest_directory, "contests/{contest_id}");
    }

    #[test]
    fn set_task_directory() {
        let mut config = default_config();
        set_value(&mut config, "task_directory", "tasks/{task_id}").unwrap();
        assert_eq!(config.task_directory, "tasks/{task_id}");
    }

    #[test]
    fn set_test_directory() {
        let mut config = default_config();
        set_value(&mut config, "test_directory", "samples").unwrap();
        assert_eq!(config.test_directory, "samples");
    }

    #[test]
    fn set_template_dir_to_path() {
        let mut config = default_config();
        set_value(&mut config, "template_dir", "/home/user/templates").unwrap();
        assert_eq!(
            config.template_dir,
            Some("/home/user/templates".to_string())
        );
    }

    #[test]
    fn set_template_dir_to_none_keyword() {
        let mut config = default_config();
        config.template_dir = Some("/some/path".to_string());
        set_value(&mut config, "template_dir", "(none)").unwrap();
        assert!(config.template_dir.is_none());
    }

    #[test]
    fn set_unknown_key_returns_error() {
        let mut config = default_config();
        assert!(set_value(&mut config, "unknown_key", "value").is_err());
    }
}
