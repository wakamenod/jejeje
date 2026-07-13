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
            println!(
                "template_dir  = {}",
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
        "template_dir" => Ok(config
            .template_dir
            .clone()
            .unwrap_or_else(|| "(none)".to_string())),
        _ => bail!("Unknown config key: '{key}'"),
    }
}

fn set_value(config: &mut Config, key: &str, value: &str) -> Result<()> {
    match key {
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
    fn get_template_dir_default() {
        let config = default_config();
        // デフォルトは ~/.config/jejeje/templates
        let expected = config.template_dir.clone().unwrap_or_else(|| "(none)".to_string());
        assert_eq!(get_value(&config, "template_dir").unwrap(), expected);
    }

    #[test]
    fn get_template_dir_none_when_explicitly_set_to_none() {
        let mut config = default_config();
        config.template_dir = None;
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
