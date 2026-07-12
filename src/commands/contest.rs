use crate::meta;
use anyhow::Result;

/// `je contest` — カレントディレクトリのコンテスト情報を表示する。
pub async fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let meta = meta::load(&cwd)?;

    println!("Contest:  {} ({})", meta.contest_name, meta.contest_id);
    println!("Judge:    {}", meta.judge);
    println!("URL:      {}", meta.url);
    println!("Tasks:    {}", meta.tasks.len());

    Ok(())
}
