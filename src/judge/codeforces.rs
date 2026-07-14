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
use std::time::Duration;
use tokio::time::sleep;

/// Codeforces へのリクエスト間の待機時間（過負荷防止）。
const REQUEST_INTERVAL: Duration = Duration::from_secs(1);

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
            filename: None,
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
///   <div class="input">
///     <div class="title">Input</div>
///     <pre>...</pre>
///   </div>
///   <div class="output">
///     <div class="title">Output</div>
///     <pre>...</pre>
///   </div>
/// </div>
/// ```
fn parse_samples(html: &str) -> Result<Vec<SampleCase>, AppError> {
    let doc = Html::parse_document(html);
    let input_sel = Selector::parse("div.sample-test div.input pre").unwrap();
    let output_sel = Selector::parse("div.sample-test div.output pre").unwrap();

    let inputs: Vec<String> = doc
        .select(&input_sel)
        .map(|el| normalize_pre_text(inner_text(el)))
        .collect();

    let outputs: Vec<String> = doc
        .select(&output_sel)
        .map(|el| normalize_pre_text(inner_text(el)))
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

/// `<pre>` 要素の内部テキストを取得する。
///
/// `scraper` の `.text()` はテキストノードのみを収集するため、
/// `<br>` や `<div>` などによる改行が失われる。
/// この関数はノードツリーを再帰的に走査し、以下のルールで `\n` を挿入する:
///
/// - `<br>` → `\n` を直接挿入
/// - `<div>`, `<p>` などのブロック要素 → 内容を取得した後に `\n` を追加
/// - `<code>`, `<span>` などのインライン要素 → 内容をそのまま取得
/// - テキストノード → そのまま追加
///
/// Codeforces の実際の HTML 例:
/// ```html
/// <pre>
///   <div class="test-example-line">7</div>
///   <div class="test-example-line">-789</div>
/// </pre>
/// ```
fn inner_text(el: scraper::ElementRef) -> String {
    use scraper::node::Node;

    /// ブロック要素として扱うタグ名（内容の後に \n を挿入する）。
    const BLOCK_TAGS: &[&str] = &["div", "p", "li", "tr", "td", "th", "h1", "h2", "h3", "h4", "h5", "h6"];

    fn traverse(node: &scraper::ElementRef, buf: &mut String) {
        for child in node.children() {
            match child.value() {
                Node::Text(t) => buf.push_str(t),
                Node::Element(e) if e.name() == "br" => buf.push('\n'),
                Node::Element(e) if BLOCK_TAGS.contains(&e.name()) => {
                    if let Some(child_el) = scraper::ElementRef::wrap(child) {
                        traverse(&child_el, buf);
                    }
                    // ブロック要素の末尾に改行を追加（まだ改行がない場合のみ）
                    if !buf.ends_with('\n') {
                        buf.push('\n');
                    }
                }
                Node::Element(_) => {
                    if let Some(child_el) = scraper::ElementRef::wrap(child) {
                        traverse(&child_el, buf);
                    }
                }
                _ => {}
            }
        }
    }

    let mut buf = String::new();
    traverse(&el, &mut buf);
    buf
}

/// `<pre>` テキストを正規化する。
///
/// - 末尾の空白・改行文字を除去し、`\n` 1 つで終わるよう統一する
/// - 空文字列の場合はそのまま返す
fn normalize_pre_text(s: String) -> String {
    let trimmed = s.trim_end_matches(['\n', '\r', ' ']);
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
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

/// URL から HTML を取得する。
///
/// リクエスト前に [`REQUEST_INTERVAL`] だけ待機し、Codeforces サーバーへの
/// 過負荷を防ぐ。
async fn fetch_html(url: &str, client: &reqwest::Client) -> Result<String, AppError> {
    sleep(REQUEST_INTERVAL).await;
    let resp = client.get(url).send().await?;
    Ok(resp.text().await?)
}

/// Codeforces API を利用してコンテスト一覧を取得する。
pub async fn fetch_contest_list(
    client: &reqwest::Client,
) -> Result<Vec<super::model::SimpleContest>, AppError> {
    let url = "https://codeforces.com/api/contest.list?gym=false";
    
    #[derive(Debug, serde::Deserialize)]
    struct CfContestListResponse {
        status: String,
        result: Vec<CfContest>,
    }

    #[derive(Debug, serde::Deserialize)]
    struct CfContest {
        id: u64,
        name: String,
    }

    let resp: CfContestListResponse = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "je-cli")
        .send()
        .await?
        .json()
        .await
        .map_err(|e| AppError::SampleParse(format!("Failed to fetch Codeforces contests: {}", e)))?;

    if resp.status != "OK" {
        return Err(AppError::SampleParse("Codeforces API status was not OK".to_string()));
    }

    let contests = resp.result
        .into_iter()
        .map(|c| super::model::SimpleContest {
            id: c.id.to_string(),
            name: c.name,
            url: format!("https://codeforces.com/contest/{}", c.id),
        })
        .collect();

    Ok(contests)
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
  <div class="input">
    <div class="title">Input</div>
    <pre>3 5</pre>
  </div>
  <div class="output">
    <div class="title">Output</div>
    <pre>8</pre>
  </div>
</div>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].input, "3 5\n");
        assert_eq!(samples[0].output, "8\n");
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
        assert_eq!(samples[0].input, "1\n");
        assert_eq!(samples[1].input, "2\n");
    }

    #[test]
    fn parse_samples_multiline_input() {
        let html = r#"
<html><body>
<div class="sample-test">
  <div class="input"><pre>3
1 2 3</pre></div>
  <div class="output"><pre>6</pre></div>
</div>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].input, "3\n1 2 3\n");
        assert_eq!(samples[0].output, "6\n");
    }

    #[test]
    fn parse_samples_trailing_newlines_normalized() {
        let html = r#"
<html><body>
<div class="sample-test">
  <div class="input"><pre>42

</pre></div>
  <div class="output"><pre>yes

</pre></div>
</div>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples[0].input, "42\n");
        assert_eq!(samples[0].output, "yes\n");
    }

    #[test]
    fn parse_samples_div_test_example_line() {
        // Codeforces の実際の HTML 構造: <pre> 内の各行が div.test-example-line で囲まれる。
        // https://codeforces.com/contest/1669/problem/A などで確認された形式。
        let html = r#"
<html><body>
<div class="sample-test">
  <div class="input">
    <div class="title">Input</div>
    <pre>
<div class="test-example-line test-example-line-even test-example-line-0">7</div><div class="test-example-line test-example-line-odd test-example-line-1">-789</div><div class="test-example-line test-example-line-even test-example-line-2">1299</div><div class="test-example-line test-example-line-odd test-example-line-3">1300</div><div class="test-example-line test-example-line-even test-example-line-4">1399</div><div class="test-example-line test-example-line-odd test-example-line-5">1400</div><div class="test-example-line test-example-line-even test-example-line-6">1679</div><div class="test-example-line test-example-line-odd test-example-line-7">2300</div></pre>
  </div>
  <div class="output">
    <div class="title">Output</div>
    <pre>
<div class="test-example-line test-example-line-odd test-example-line-1">Division 4</div><div class="test-example-line test-example-line-even test-example-line-2">Division 4</div><div class="test-example-line test-example-line-odd test-example-line-3">Division 4</div><div class="test-example-line test-example-line-even test-example-line-4">Division 4</div><div class="test-example-line test-example-line-odd test-example-line-5">Division 3</div><div class="test-example-line test-example-line-even test-example-line-6">Division 2</div><div class="test-example-line test-example-line-odd test-example-line-7">Division 1</div></pre>
  </div>
</div>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(
            samples[0].input,
            "7\n-789\n1299\n1300\n1399\n1400\n1679\n2300\n"
        );
        assert_eq!(
            samples[0].output,
            "Division 4\nDivision 4\nDivision 4\nDivision 4\nDivision 3\nDivision 2\nDivision 1\n"
        );
    }

    #[test]
    fn parse_samples_br_newlines() {
        // Codeforces の実際の HTML では <pre> 内の改行が <br> で表現される場合がある。
        // scraper の .text() は <br> を無視するため inner_text() で明示的に \n へ変換する。
        let html = r#"
<html><body>
<div class="sample-test">
  <div class="input">
    <div class="title">Input</div>
    <pre>7<br/>-789<br/>1299<br/>1300<br/>1399<br/>1400<br/>1679<br/>2300</pre>
  </div>
  <div class="output">
    <div class="title">Output</div>
    <pre>YES<br/>YES<br/>NO<br/>YES<br/>NO<br/>YES<br/>YES</pre>
  </div>
</div>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].input, "7\n-789\n1299\n1300\n1399\n1400\n1679\n2300\n");
        assert_eq!(samples[0].output, "YES\nYES\nNO\nYES\nNO\nYES\nYES\n");
    }

    #[test]
    fn parse_samples_no_samples_returns_error() {
        let html = "<html><body><p>No samples</p></body></html>";
        let err = parse_samples(html).unwrap_err();
        assert!(matches!(err, AppError::SampleParse(_)));
    }

    // ─── normalize_pre_text ──────────────────────────────────────

    #[test]
    fn normalize_pre_text_adds_trailing_newline() {
        assert_eq!(normalize_pre_text("3 5".to_string()), "3 5\n");
    }

    #[test]
    fn normalize_pre_text_strips_extra_trailing_newlines() {
        assert_eq!(normalize_pre_text("8\n\n".to_string()), "8\n");
    }

    #[test]
    fn normalize_pre_text_strips_trailing_spaces() {
        assert_eq!(normalize_pre_text("hello   ".to_string()), "hello\n");
    }

    #[test]
    fn normalize_pre_text_preserves_internal_newlines() {
        assert_eq!(normalize_pre_text("1 2\n3 4\n".to_string()), "1 2\n3 4\n");
    }

    #[test]
    fn normalize_pre_text_empty_string() {
        assert_eq!(normalize_pre_text(String::new()), "");
    }

    #[test]
    fn normalize_pre_text_only_whitespace() {
        assert_eq!(normalize_pre_text("   \n\n".to_string()), "");
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
