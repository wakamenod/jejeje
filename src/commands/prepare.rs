use crate::{config::Config, judge, meta};
use anyhow::{Context, Result};
use std::{fs, path::Path};

/// `je prepare <url>` — コンテスト URL または問題 URL からディレクトリをセットアップする。
///
/// - コンテスト URL: 全タスクのディレクトリを一括作成し `.je-meta.json` を保存する
/// - 問題 URL: 単一タスクのディレクトリを作成する
///
/// サンプルファイル (test/*.in / test/*.out) は常に最新に更新する。
/// テンプレートファイルはコピー先が存在する場合はスキップする（回答中のファイルを保護）。
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
            setup_task_dir(&task_dir, &task.url, &config, &client, &template).await?;
            println!("  [{}] {} — {}", task.id, task.name, task_dir.display());
        }
    } else {
        // 問題 URL: 単一タスクのディレクトリを作成
        let cwd = std::env::current_dir()?;
        let base_dir = meta::find_contest_root(&cwd).unwrap_or_else(|| cwd.clone());

        let task_id = infer_task_id(&url);
        let task_dir = base_dir.join(&task_id);

        println!("Preparing task '{task_id}'...");
        setup_task_dir(&task_dir, &url, &config, &client, &template)
            .await
            .with_context(|| format!("Failed to prepare task directory '{}'", task_dir.display()))?;

        println!("Done: {}", task_dir.display());
    }

    Ok(())
}

/// タスクディレクトリをセットアップする。
///
/// - `test/` ディレクトリを作成し、サンプルファイルを常に上書きする
/// - テンプレートファイルはコピー先が存在しない場合のみコピーする
pub async fn setup_task_dir(
    task_dir: &Path,
    problem_url: &str,
    config: &Config,
    client: &reqwest::Client,
    template: &Option<String>,
) -> Result<()> {
    let test_dir = task_dir.join(&config.test_directory);
    fs::create_dir_all(&test_dir)?;

    // サンプルは常に最新に更新する
    let samples = judge::fetch_samples(problem_url, client).await?;
    if samples.is_empty() {
        println!("  (no samples found)");
    }
    for (i, sample) in samples.iter().enumerate() {
        let n = i + 1;
        fs::write(test_dir.join(format!("{n}.in")), &sample.input)?;
        fs::write(test_dir.join(format!("{n}.out")), &sample.output)?;
    }

    // テンプレートは既存ファイルを上書きしない
    let tmpl_name = template.as_ref().or(config.default_template.as_ref());
    if let Some(name) = tmpl_name {
        copy_template_safe(task_dir, name, config)?;
    }

    Ok(())
}

/// テンプレートディレクトリの内容をタスクディレクトリへコピーする。
/// コピー先のファイルがすでに存在する場合はスキップする。
fn copy_template_safe(task_dir: &Path, template_name: &str, config: &Config) -> Result<()> {
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
            if dest.exists() {
                // 既存ファイルは保護する（回答中のコードを上書きしない）
                continue;
            }
            fs::copy(entry.path(), &dest)?;
        }
    }
    Ok(())
}

/// URL からタスク ID を推定する。
///
/// 例: `https://atcoder.jp/contests/abc001/tasks/abc001_a` → `"abc001_a"`
fn infer_task_id(url: &str) -> String {
    url.trim_end_matches('/')
        .split('/')
        .last()
        .unwrap_or("task")
        .to_string()
}

pub fn build_client() -> Result<reqwest::Client> {
    Ok(reqwest::Client::builder()
        .cookie_store(true)
        .user_agent(concat!("je/", env!("CARGO_PKG_VERSION")))
        .build()?)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn infer_task_id_atcoder() {
        assert_eq!(
            infer_task_id("https://atcoder.jp/contests/abc001/tasks/abc001_a"),
            "abc001_a"
        );
    }

    #[test]
    fn infer_task_id_codeforces() {
        assert_eq!(
            infer_task_id("https://codeforces.com/contest/1234/problem/A"),
            "A"
        );
    }

    #[test]
    fn infer_task_id_trailing_slash() {
        assert_eq!(
            infer_task_id("https://atcoder.jp/contests/abc001/tasks/abc001_b/"),
            "abc001_b"
        );
    }

    #[test]
    fn infer_task_id_yukicoder() {
        assert_eq!(
            infer_task_id("https://yukicoder.me/problems/no/42"),
            "42"
        );
    }

    #[test]
    fn infer_task_id_empty_string() {
        assert_eq!(infer_task_id(""), "");
    }
}
