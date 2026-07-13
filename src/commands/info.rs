use crate::meta;
use anyhow::Result;

/// `je info` — カレントディレクトリのコンテスト情報とタスク一覧を表示する。
pub async fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let meta = meta::load(&cwd)?;

    println!("Contest:  {} ({})", meta.contest_name, meta.contest_id);
    println!("Judge:    {}", meta.judge);
    println!("URL:      {}", meta.url);
    println!("Tasks:");

    for task in &meta.tasks {
        println!("  [{:>3}]  {}  {}", task.id, task.name, task.url);
    }

    Ok(())
}
