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

#[cfg(test)]
mod tests {
    use super::*;

    // ─── URL 判定 ─────────────────────────────────────────────────

    #[test]
    fn is_url_contest() {
        assert!(is_url("https://codeforces.com/contest/1234"));
    }

    #[test]
    fn is_url_gym() {
        assert!(is_url("https://codeforces.com/gym/102028"));
    }

    #[test]
    fn is_url_false_for_other() {
        assert!(!is_url("https://atcoder.jp/contests/abc001"));
    }

    #[test]
    fn is_contest_url_contest() {
        assert!(is_contest_url("https://codeforces.com/contest/1234"));
    }

    #[test]
    fn is_contest_url_gym() {
        assert!(is_contest_url("https://codeforces.com/gym/102028"));
    }

    #[test]
    fn is_contest_url_false_for_problem() {
        assert!(!is_contest_url(
            "https://codeforces.com/contest/1234/problem/A"
        ));
    }

    #[test]
    fn is_problem_url_problem() {
        assert!(is_problem_url(
            "https://codeforces.com/contest/1234/problem/A"
        ));
    }

    #[test]
    fn is_problem_url_problemset() {
        assert!(is_problem_url(
            "https://codeforces.com/problemset/problem/1234/A"
        ));
    }

    #[test]
    fn is_problem_url_false_for_contest() {
        assert!(!is_problem_url("https://codeforces.com/contest/1234"));
    }

    // ─── extract_contest_id ──────────────────────────────────────

    #[test]
    fn extract_contest_id_contest() {
        let id = extract_contest_id("https://codeforces.com/contest/1234").unwrap();
        assert_eq!(id, "1234");
    }

    #[test]
    fn extract_contest_id_gym() {
        let id = extract_contest_id("https://codeforces.com/gym/102028").unwrap();
        assert_eq!(id, "102028");
    }

    #[test]
    fn extract_contest_id_from_problem_url() {
        let id =
            extract_contest_id("https://codeforces.com/contest/1234/problem/A").unwrap();
        assert_eq!(id, "1234");
    }

    #[test]
    fn extract_contest_id_trailing_slash() {
        let id = extract_contest_id("https://codeforces.com/contest/1234/").unwrap();
        assert_eq!(id, "1234");
    }

    #[test]
    fn extract_contest_id_unsupported() {
        let err = extract_contest_id("https://example.com/foo").unwrap_err();
        assert!(matches!(err, AppError::UnsupportedUrl(_)));
    }

    // ─── parse_contest_problems ──────────────────────────────────

    #[test]
    fn parse_contest_problems_basic() {
        let html = r#"
<html><body>
<table class="problems">
  <tr>
    <td>A</td>
    <td><a href="/contest/1234/problem/A">Hello World</a></td>
  </tr>
  <tr>
    <td>B</td>
    <td><a href="/contest/1234/problem/B">Two Sum</a></td>
  </tr>
</table>
</body></html>
"#;
        let tasks = parse_contest_problems(html, "1234", "contest").unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "a");
        assert_eq!(tasks[0].name, "Hello World");
        assert!(tasks[0].url.contains("/problem/A"));
        assert_eq!(tasks[1].id, "b");
    }

    #[test]
    fn parse_contest_problems_empty_returns_error() {
        let html = "<html><body></body></html>";
        let err = parse_contest_problems(html, "1234", "contest").unwrap_err();
        assert!(matches!(err, AppError::SampleParse(_)));
    }

    // ─── parse_samples ───────────────────────────────────────────

    #[test]
    fn parse_samples_basic() {
        let html = r#"
<html><body>
<div class="sample-test">
  <div class="input"><pre>3 5</pre></div>
  <div class="output"><pre>8</pre></div>
</div>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].input.trim(), "3 5");
        assert_eq!(samples[0].output.trim(), "8");
    }

    #[test]
    fn parse_samples_multiple() {
        let html = r#"
<html><body>
<div class="sample-test">
  <div class="input"><pre>1</pre></div>
  <div class="output"><pre>1</pre></div>
  <div class="input"><pre>2</pre></div>
  <div class="output"><pre>4</pre></div>
</div>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 2);
    }

    #[test]
    fn parse_samples_no_samples_returns_error() {
        let html = "<html><body><p>No samples</p></body></html>";
        let err = parse_samples(html).unwrap_err();
        assert!(matches!(err, AppError::SampleParse(_)));
    }

    // ─── parse_contest_name ──────────────────────────────────────

    #[test]
    fn parse_contest_name_found() {
        let html = r#"<html><body><div class="contest-name">Codeforces Round #750</div></body></html>"#;
        let name = parse_contest_name(html).unwrap();
        assert_eq!(name, "Codeforces Round #750");
    }

    #[test]
    fn parse_contest_name_not_found() {
        let html = "<html><body></body></html>";
        assert!(parse_contest_name(html).is_none());
    }
}
