use crate::{config::Config, judge, meta};
use anyhow::{Context, Result};
use std::{fs, path::Path};

/// `je new <url>` — コンテスト URL からディレクトリを構築してサンプルを取得する。
pub async fn run(url: String, template: Option<String>) -> Result<()> {
    let config = Config::load()?;
    let client = build_client()?;

    if judge::is_contest_url(&url) {
        // コンテスト URL: 全タスクのディレクトリを一括作成
        println!("Fetching contest info from {url}...");
        let contest_meta = judge::fetch_contest(&url, &client).await?;

        let contest_dir = Path::new(&contest_meta.contest_id).to_path_buf();
        fs::create_dir_all(&contest_dir)
            .with_context(|| format!("Failed to create {}", contest_dir.display()))?;

        // メタデータ保存
        meta::save(&contest_dir, &contest_meta)?;
        println!(
            "Contest: {} ({})",
            contest_meta.contest_name, contest_meta.contest_id
        );

        for task in &contest_meta.tasks {
            let task_dir = contest_dir.join(&task.id);
            create_task_dir(&task_dir, &task.url, &config, &client, &template).await?;
            println!("  [{}] {} — {}", task.id, task.name, task_dir.display());
        }
    } else {
        // 問題 URL として扱う（add と同じ動作）
        return crate::commands::add::run(url, template).await;
    }

    Ok(())
}

/// タスクディレクトリを作成し、サンプルをダウンロードしてテンプレートをコピーする。
pub async fn create_task_dir(
    task_dir: &Path,
    problem_url: &str,
    config: &Config,
    client: &reqwest::Client,
    template: &Option<String>,
) -> Result<()> {
    let test_dir = task_dir.join(&config.test_directory);
    fs::create_dir_all(&test_dir)?;

    // サンプルをダウンロード
    let samples = judge::fetch_samples(problem_url, client).await?;
    for (i, sample) in samples.iter().enumerate() {
        let n = i + 1;
        fs::write(test_dir.join(format!("{n}.in")), &sample.input)?;
        fs::write(test_dir.join(format!("{n}.out")), &sample.output)?;
    }

    // テンプレートをコピー
    let tmpl_name = template.as_ref().or(config.default_template.as_ref());
    if let Some(name) = tmpl_name {
        copy_template(task_dir, name, config)?;
    }

    Ok(())
}

/// テンプレートディレクトリの内容をタスクディレクトリへコピーする。
fn copy_template(task_dir: &Path, template_name: &str, config: &Config) -> Result<()> {
    let template_base = match &config.template_dir {
        Some(dir) => Path::new(dir).to_path_buf(),
        None => return Ok(()), // template_dir 未設定
    };
    let template_dir = template_base.join(template_name);
    if !template_dir.exists() {
        anyhow::bail!(
            "Template '{}' not found at {}",
            template_name,
            template_dir.display()
        );
    }
    for entry in fs::read_dir(&template_dir)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let dest = task_dir.join(entry.file_name());
            fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

pub fn build_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .cookie_store(true)
        .user_agent(concat!("je/", env!("CARGO_PKG_VERSION")))
        .build()?)
}
