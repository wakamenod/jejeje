use crate::{config::Config, meta};
use anyhow::{Context, Result};

/// `je add <url>` — 問題 URL から単一タスクのディレクトリを作成してサンプルを取得する。
///
/// 現在のディレクトリまたは親ディレクトリに `.je-meta.json` があれば、
/// コンテストディレクトリ内にタスクを追加する。
pub async fn run(url: String, template: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let client = crate::commands::new::build_client()?;

    // カレントディレクトリからコンテストルートを探す
    let cwd = std::env::current_dir()?;
    let base_dir = meta::find_contest_root(&cwd).unwrap_or_else(|| cwd.clone());

    // URL からタスク ID を推定（judge 実装に委譲）
    let task_id = infer_task_id(&url);
    let task_dir = base_dir.join(&task_id);

    println!("Adding task '{task_id}'...");
    crate::commands::new::create_task_dir(&task_dir, &url, &config, &client, &template)
        .await
        .with_context(|| format!("Failed to create task directory '{}'", task_dir.display()))?;

    println!("Created: {}", task_dir.display());
    Ok(())
}

/// URL から簡易的にタスク ID を推定する。
///
/// 例: `https://atcoder.jp/contests/abc001/tasks/abc001_a` → `"abc001_a"`
fn infer_task_id(url: &str) -> String {
    url.trim_end_matches('/')
        .split('/')
        .last()
        .unwrap_or("task")
        .to_string()
}
