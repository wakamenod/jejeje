//! Codeforces のサンプル取得・コンテスト情報取得。
//!
//! # URL パターン
//! - コンテスト: `https://codeforces.com/contest/{contest_id}`
//! - 問題:       `https://codeforces.com/contest/{contest_id}/problem/{problem_id}`
//!               `https://codeforces.com/problemset/problem/{contest_id}/{problem_id}`
//! - Gym:        `https://codeforces.com/gym/{contest_id}`
//!               `https://codeforces.com/gym/{contest_id}/problem/{problem_id}`

use super::model::{ContestMeta, SampleCase, TaskMeta};
use crate::error::AppError;
use scraper::{Html, Selector};

const BASE: &str = "https://codeforces.com";

// ─── URL 判定 ──────────────────────────────────────────────────────

pub fn is_url(url: &str) -> bool {
    url.contains("codeforces.com")
}

pub fn is_contest_url(url: &str) -> bool {
    is_url(url)
        && (url.contains("/contest/") || url.contains("/gym/"))
        && !url.contains("/problem/")
}

pub fn is_problem_url(url: &str) -> bool {
    is_url(url)
        && (url.contains("/problem/") || url.contains("/problemset/problem/"))
}

// ─── コンテスト取得 ─────────────────────────────────────────────────

pub async fn fetch_contest(
    url: &str,
    client: &reqwest::Client,
) -> Result<ContestMeta, AppError> {
    let contest_id = extract_contest_id(url)?;
    let is_gym = url.contains("/gym/");
    let base_path = if is_gym { "gym" } else { "contest" };

    let html = fetch_html(url, client).await?;
    let tasks = parse_contest_problems(&html, &contest_id, base_path)?;
    let contest_name = parse_contest_name(&html).unwrap_or_else(|| contest_id.clone());

    Ok(ContestMeta {
        judge: "codeforces".to_string(),
        contest_id,
        contest_name,
        url: url.to_string(),
        tasks,
    })
}

// ─── サンプル取得 ───────────────────────────────────────────────────

pub async fn fetch_samples(
    url: &str,
    client: &reqwest::Client,
) -> Result<Vec<SampleCase>, AppError> {
    let html = fetch_html(url, client).await?;
    parse_samples(&html)
}

// ─── パース ────────────────────────────────────────────────────────

/// コンテストページの問題テーブルをパースする。
///
/// Codeforces の問題一覧テーブルは `.problems` クラスを持つ `<table>` に含まれる。
fn parse_contest_problems(
    html: &str,
    contest_id: &str,
    base_path: &str,
) -> Result<Vec<TaskMeta>, AppError> {
    let doc = Html::parse_document(html);
    let row_sel = Selector::parse("table.problems tr").unwrap();
    let td_sel = Selector::parse("td").unwrap();
    let a_sel = Selector::parse("a").unwrap();

    let mut tasks = Vec::new();

    for row in doc.select(&row_sel) {
        let cols: Vec<_> = row.select(&td_sel).collect();
        if cols.len() < 2 {
            continue;
        }

        // 1 列目: 問題 ID (A, B, C, ...)
        let id = cols[0].text().collect::<String>().trim().to_string();
        if id.is_empty() {
            continue;
        }

        // 2 列目: 問題名とリンク
        let name_cell = &cols[1];
        let name = name_cell
            .select(&a_sel)
            .next()
            .map(|a| a.text().collect::<String>().trim().to_string())
            .unwrap_or_default();

        let task_url = format!("{BASE}/{base_path}/{contest_id}/problem/{id}");

        tasks.push(TaskMeta {
            id: id.to_lowercase(),
            name,
            url: task_url,
        });
    }

    if tasks.is_empty() {
        return Err(AppError::SampleParse(format!(
            "No problems found for contest '{contest_id}'"
        )));
    }

    Ok(tasks)
}

/// Codeforces 問題ページの `<div class="sample-test">` からサンプルを取得する。
///
/// 構造:
/// ```html
/// <div class="sample-test">
///   <div class="input"><pre>...</pre></div>
///   <div class="output"><pre>...</pre></div>
/// </div>
/// ```
fn parse_samples(html: &str) -> Result<Vec<SampleCase>, AppError> {
    let doc = Html::parse_document(html);
    let input_sel = Selector::parse("div.sample-test div.input pre").unwrap();
    let output_sel = Selector::parse("div.sample-test div.output pre").unwrap();

    let inputs: Vec<String> = doc
        .select(&input_sel)
        .map(|el| el.text().collect::<String>())
        .collect();

    let outputs: Vec<String> = doc
        .select(&output_sel)
        .map(|el| el.text().collect::<String>())
        .collect();

    if inputs.is_empty() {
        return Err(AppError::SampleParse(
            "No sample inputs found on this page".to_string(),
        ));
    }

    Ok(inputs
        .into_iter()
        .zip(outputs)
        .map(|(input, output)| SampleCase { input, output })
        .collect())
}

fn parse_contest_name(html: &str) -> Option<String> {
    let doc = Html::parse_document(html);
    let sel = Selector::parse(".contest-name").unwrap();
    doc.select(&sel)
        .next()
        .map(|el| el.text().collect::<String>().trim().to_string())
        .filter(|s| !s.is_empty())
}

// ─── ヘルパー ──────────────────────────────────────────────────────

fn extract_contest_id(url: &str) -> Result<String, AppError> {
    // /contest/1234 または /gym/1234 の数字部分を抽出
    let trimmed = url.trim_end_matches('/');
    let segment = if let Some(s) = trimmed.split("/contest/").nth(1) {
        s
    } else if let Some(s) = trimmed.split("/gym/").nth(1) {
        s
    } else {
        return Err(AppError::UnsupportedUrl(url.to_string()));
    };
    Ok(segment.split('/').next().unwrap_or("").to_string())
}

async fn fetch_html(url: &str, client: &reqwest::Client) -> Result<String, AppError> {
    let resp = client.get(url).send().await?;
    Ok(resp.text().await?)
}
