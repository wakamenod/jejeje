use crate::meta;
use anyhow::Result;

/// `je tasks` — コンテスト内のタスク一覧を表示する。
pub async fn run() -> Result<()> {
    let cwd = std::env::current_dir()?;
    let meta = meta::load(&cwd)?;

    println!("Tasks in {} ({}):", meta.contest_name, meta.contest_id);
    println!();

    for task in &meta.tasks {
        println!("  [{:>3}]  {}  {}", task.id, task.name, task.url);
    }

    Ok(())
}
