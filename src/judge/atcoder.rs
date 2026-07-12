//! AtCoder のサンプル取得・コンテスト情報取得。
//!
//! # URL パターン
//! - コンテスト: `https://atcoder.jp/contests/{contest_id}`
//! - 問題:       `https://atcoder.jp/contests/{contest_id}/tasks/{task_id}`

use super::model::{ContestMeta, SampleCase, TaskMeta};
use crate::error::AppError;
use scraper::{Html, Selector};

const BASE: &str = "https://atcoder.jp";

// ─── URL 判定 ──────────────────────────────────────────────────────

/// AtCoder のコンテスト URL か問題 URL のいずれかであれば `true`。
pub fn is_url(url: &str) -> bool {
    url.contains("atcoder.jp/contests/")
}

/// コンテスト URL（タスク URL ではない）なら `true`。
///
/// 例: `https://atcoder.jp/contests/abc001`
pub fn is_contest_url(url: &str) -> bool {
    is_url(url) && !url.contains("/tasks")
}

/// 問題 URL なら `true`。
///
/// 例: `https://atcoder.jp/contests/abc001/tasks/abc001_a`
pub fn is_problem_url(url: &str) -> bool {
    is_url(url) && url.contains("/tasks/")
}

// ─── コンテスト取得 ─────────────────────────────────────────────────

/// コンテスト URL からタスク一覧を含むメタデータを取得する。
pub async fn fetch_contest(
    url: &str,
    client: &reqwest::Client,
) -> Result<ContestMeta, AppError> {
    // コンテスト ID を URL から抽出
    // e.g. https://atcoder.jp/contests/abc001 → "abc001"
    let contest_id = extract_contest_id(url)?;

    // タスク一覧ページを取得
    let tasks_url = format!("{BASE}/contests/{contest_id}/tasks");
    let html = fetch_html(&tasks_url, client).await?;

    let tasks = parse_task_table(&html, &contest_id)?;

    // コンテスト名はトップページのタイトルから取得
    let top_html = fetch_html(&format!("{BASE}/contests/{contest_id}"), client).await?;
    let contest_name = parse_contest_name(&top_html).unwrap_or_else(|| contest_id.clone());

    Ok(ContestMeta {
        judge: "atcoder".to_string(),
        contest_id,
        contest_name,
        url: url.to_string(),
        tasks,
    })
}

// ─── サンプル取得 ───────────────────────────────────────────────────

/// 問題 URL からサンプルケース一覧を取得する。
pub async fn fetch_samples(
    url: &str,
    client: &reqwest::Client,
) -> Result<Vec<SampleCase>, AppError> {
    let html = fetch_html(url, client).await?;
    parse_samples(&html)
}

// ─── パース ────────────────────────────────────────────────────────

/// タスク一覧テーブルをパースして `Vec<TaskMeta>` を返す。
///
/// AtCoder のタスクテーブルは `#task-table` に含まれており、
/// 各行の 1 列目がアルファベット、2 列目がタスク名とリンク。
fn parse_task_table(html: &str, contest_id: &str) -> Result<Vec<TaskMeta>, AppError> {
    let doc = Html::parse_document(html);
    let row_sel = Selector::parse("#task-table tbody tr").unwrap();
    let td_sel = Selector::parse("td").unwrap();
    let a_sel = Selector::parse("a").unwrap();

    let mut tasks = Vec::new();

    for row in doc.select(&row_sel) {
        let cols: Vec<_> = row.select(&td_sel).collect();
        if cols.len() < 2 {
            continue;
        }

        let id = cols[0].text().collect::<String>().trim().to_lowercase();
        let name_cell = &cols[1];
        let name = name_cell.text().collect::<String>().trim().to_string();
        let href = name_cell
            .select(&a_sel)
            .next()
            .and_then(|a| a.value().attr("href"))
            .unwrap_or("");

        let task_url = if href.starts_with("http") {
            href.to_string()
        } else {
            format!("{BASE}{href}")
        };

        tasks.push(TaskMeta {
            id,
            name,
            url: task_url,
        });
    }

    if tasks.is_empty() {
        return Err(AppError::SampleParse(format!(
            "No tasks found for contest '{contest_id}'"
        )));
    }

    Ok(tasks)
}

/// 問題ページから入力例・出力例の `<pre>` ブロックをペアリングして返す。
///
/// AtCoder の問題ページでは `<section>` の `<h3>` タグに
/// "入力例" / "出力例" または "Sample Input" / "Sample Output" が含まれる。
fn parse_samples(html: &str) -> Result<Vec<SampleCase>, AppError> {
    let doc = Html::parse_document(html);
    let section_sel = Selector::parse("section").unwrap();
    let h3_sel = Selector::parse("h3").unwrap();
    let pre_sel = Selector::parse("pre").unwrap();

    let mut inputs: Vec<String> = Vec::new();
    let mut outputs: Vec<String> = Vec::new();

    for section in doc.select(&section_sel) {
        let heading = section
            .select(&h3_sel)
            .next()
            .map(|h| h.text().collect::<String>())
            .unwrap_or_default();

        let pre_text = section
            .select(&pre_sel)
            .next()
            .map(|p| p.text().collect::<String>())
            .unwrap_or_default();

        if heading.contains("入力例") || heading.contains("Sample Input") {
            inputs.push(pre_text);
        } else if heading.contains("出力例") || heading.contains("Sample Output") {
            outputs.push(pre_text);
        }
    }

    if inputs.is_empty() {
        return Err(AppError::SampleParse(
            "No sample inputs found on this page".to_string(),
        ));
    }

    let samples = inputs
        .into_iter()
        .zip(outputs)
        .map(|(input, output)| SampleCase { input, output })
        .collect();

    Ok(samples)
}

/// `<title>` タグからコンテスト名を抽出する。
fn parse_contest_name(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let title_sel = Selector::parse("title").unwrap();
    doc.select(&title_sel)
        .next()
        .map(|t| t.text().collect::<String>())
        .map(|t| t.split('-').next().unwrap_or("").trim().to_string())
        .filter(|s| !s.is_empty())
}

// ─── ヘルパー ──────────────────────────────────────────────────────

fn extract_contest_id(url: &str) -> Result<String, AppError> {
    // "https://atcoder.jp/contests/abc001" → "abc001"
    url.trim_end_matches('/')
        .split("/contests/")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::UnsupportedUrl(url.to_string()))
}

async fn fetch_html(url: &str, client: &reqwest::Client) -> Result<String, AppError> {
    let resp = client.get(url).send().await?;
    Ok(resp.text().await?)
}
