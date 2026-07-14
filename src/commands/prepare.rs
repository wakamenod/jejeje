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
pub async fn run(url_or_query: String) -> Result<()> {
    let config = Config::load()?;
    let client = build_client()?;

    let url = judge::resolve_query(&url_or_query, &client)
        .await
        .context("Failed to resolve contest query")?;

    if judge::is_contest_url(&url) {
        // コンテスト URL: 全タスクのディレクトリを一括作成
        println!("Fetching contest info from {url}...");
        let mut contest_meta = judge::fetch_contest(&url, &client).await?;

        let contest_dir = Path::new(&contest_meta.contest_id).to_path_buf();
        fs::create_dir_all(&contest_dir)
            .with_context(|| format!("Failed to create {}", contest_dir.display()))?;

        // メタデータ保存
        meta::save(&contest_dir, &contest_meta)?;
        println!(
            "Contest: {} ({})",
            contest_meta.contest_name, contest_meta.contest_id
        );

        let mut meta_updated = false;
        for task in &mut contest_meta.tasks {
            let task_dir = contest_dir.join(&task.id);
            let copied_file = setup_task_dir(&task_dir, &task.url, &config, &client).await?;
            if let Some(fname) = copied_file {
                task.filename = Some(fname);
                meta_updated = true;
            }
            println!("  [{}] {} — {}", task.id, task.name, task_dir.display());
        }
        // テンプレートファイル名を記録した場合はメタデータを再保存する
        if meta_updated {
            meta::save(&contest_dir, &contest_meta)?;
        }
    } else {
        // 問題 URL: 単一タスクのディレクトリを作成
        let cwd = std::env::current_dir()?;
        let base_dir = meta::find_contest_root(&cwd).unwrap_or_else(|| cwd.clone());

        let task_id = infer_task_id(&url);
        let task_dir = base_dir.join(&task_id);

        println!("Preparing task '{task_id}'...");
        let copied_file = setup_task_dir(&task_dir, &url, &config, &client)
            .await
            .with_context(|| {
                format!("Failed to prepare task directory '{}'", task_dir.display())
            })?;

        // コンテストメタがあればファイル名を更新して再保存する
        if let Some(fname) = copied_file
            && let Ok(mut contest_meta) = meta::load(&cwd)
            && let Some(task) = contest_meta.tasks.iter_mut().find(|t| t.id == task_id)
        {
            task.filename = Some(fname);
            let root = meta::find_contest_root(&cwd).unwrap_or_else(|| cwd.clone());
            meta::save(&root, &contest_meta)?;
        }

        println!("Done: {}", task_dir.display());
    }

    Ok(())
}

/// タスクディレクトリをセットアップする。
///
/// - `test/` ディレクトリを作成し、サンプルファイルを常に上書きする
/// - `template_dir` が設定されている場合、その直下のファイルを全てコピーする
///   （コピー先が既に存在するファイルはスキップする）
///
/// 戻り値: 新規コピーされたテンプレートファイルの最初のファイル名（なければ `None`）
pub async fn setup_task_dir(
    task_dir: &Path,
    problem_url: &str,
    config: &Config,
    client: &reqwest::Client,
) -> Result<Option<String>> {
    let test_dir = task_dir.join("test");
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

    // template_dir 直下のファイルを全てコピーする（既存ファイルはスキップ）
    let copied_filename = if let Some(dir) = &config.template_dir {
        let copied = copy_template_all(task_dir, dir)?;
        copied.into_iter().next()
    } else {
        None
    };

    Ok(copied_filename)
}

/// `template_dir` 直下のファイルをタスクディレクトリへ全てコピーする。
/// コピー先のファイルがすでに存在する場合はスキップする（回答中のコードを保護）。
///
/// 戻り値: 新規コピーされたファイル名のリスト（ソート済み）。
fn copy_template_all(task_dir: &Path, template_dir: &str) -> Result<Vec<String>> {
    let src = Path::new(template_dir);
    if !src.exists() {
        anyhow::bail!("template_dir '{}' not found", template_dir);
    }
    let mut copied = Vec::new();
    for entry in fs::read_dir(src)? {
        let entry = entry?;
        if entry.file_type()?.is_file() {
            let dest = task_dir.join(entry.file_name());
            if dest.exists() {
                // 既存ファイルは保護する（回答中のコードを上書きしない）
                continue;
            }
            fs::copy(entry.path(), &dest)?;
            copied.push(entry.file_name().to_string_lossy().to_string());
        }
    }
    copied.sort();
    Ok(copied)
}

/// URL からタスク ID を推定する。
///
/// 例:
/// - `https://atcoder.jp/contests/abc001/tasks/abc001_a` → `"abc001_a"`
/// - `https://judge.u-aizu.ac.jp/onlinejudge/description.jsp?id=ITP1_1_A` → `"ITP1_1_A"`
fn infer_task_id(url: &str) -> String {
    // AOJ 旧形式: description.jsp?id=XXX
    if url.contains("description.jsp")
        && let Some(id) = url.split("id=").nth(1).and_then(|s| s.split('&').next())
    {
        return id.to_string();
    }

    url.trim_end_matches('/')
        .split('/')
        .next_back()
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
    use std::fs;
    use tempfile::tempdir;

    // ─── copy_template_all ────────────────────────────────────────

    #[test]
    fn copy_template_all_copies_files() {
        let template_dir = tempdir().unwrap();
        let task_dir = tempdir().unwrap();

        fs::write(template_dir.path().join("main.rs"), "fn main() {}").unwrap();

        let copied =
            copy_template_all(task_dir.path(), template_dir.path().to_str().unwrap()).unwrap();

        let dest = task_dir.path().join("main.rs");
        assert!(dest.exists());
        assert_eq!(fs::read_to_string(dest).unwrap(), "fn main() {}");
        assert_eq!(copied, vec!["main.rs"]);
    }

    #[test]
    fn copy_template_all_does_not_overwrite_existing_file() {
        let template_dir = tempdir().unwrap();
        let task_dir = tempdir().unwrap();

        fs::write(template_dir.path().join("main.rs"), "template content").unwrap();
        // タスクディレクトリにすでに同名ファイルが存在する
        fs::write(task_dir.path().join("main.rs"), "my solution").unwrap();

        let copied =
            copy_template_all(task_dir.path(), template_dir.path().to_str().unwrap()).unwrap();

        // 既存ファイルが上書きされていないことを確認
        let content = fs::read_to_string(task_dir.path().join("main.rs")).unwrap();
        assert_eq!(content, "my solution");
        // スキップされたのでコピーリストは空
        assert!(copied.is_empty());
    }

    #[test]
    fn copy_template_all_copies_new_files_and_skips_existing() {
        let template_dir = tempdir().unwrap();
        let task_dir = tempdir().unwrap();

        fs::write(template_dir.path().join("main.rs"), "template main").unwrap();
        fs::write(template_dir.path().join("Cargo.toml"), "[package]").unwrap();
        // main.rs だけ既存
        fs::write(task_dir.path().join("main.rs"), "my solution").unwrap();

        let copied =
            copy_template_all(task_dir.path(), template_dir.path().to_str().unwrap()).unwrap();

        // main.rs は上書きされない
        assert_eq!(
            fs::read_to_string(task_dir.path().join("main.rs")).unwrap(),
            "my solution"
        );
        // Cargo.toml は新規コピーされる
        assert_eq!(
            fs::read_to_string(task_dir.path().join("Cargo.toml")).unwrap(),
            "[package]"
        );
        // コピーされたのは Cargo.toml のみ
        assert_eq!(copied, vec!["Cargo.toml"]);
    }

    #[test]
    fn copy_template_all_returns_error_when_template_dir_missing() {
        let task_dir = tempdir().unwrap();
        let result = copy_template_all(task_dir.path(), "/nonexistent/template/dir");
        assert!(result.is_err());
    }

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
        assert_eq!(infer_task_id("https://yukicoder.me/problems/no/42"), "42");
    }

    #[test]
    fn infer_task_id_empty_string() {
        assert_eq!(infer_task_id(""), "");
    }

    #[test]
    fn infer_task_id_aoj_description_jsp() {
        assert_eq!(
            infer_task_id("https://judge.u-aizu.ac.jp/onlinejudge/description.jsp?id=ITP1_1_A"),
            "ITP1_1_A"
        );
    }

    #[test]
    fn infer_task_id_aoj_description_jsp_with_extra_param() {
        assert_eq!(
            infer_task_id("https://judge.u-aizu.ac.jp/onlinejudge/description.jsp?id=0001&lang=en"),
            "0001"
        );
    }
}
