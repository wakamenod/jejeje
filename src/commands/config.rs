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
                "default_template   = {}",
                config.default_template.as_deref().unwrap_or("(none)")
            );
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
        "default_template" => Ok(config
            .default_template
            .clone()
            .unwrap_or_else(|| "(none)".to_string())),
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
        "default_template" => {
            config.default_template = if value == "(none)" || value.is_empty() {
                None
            } else {
                Some(value.to_string())
            }
        }
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
