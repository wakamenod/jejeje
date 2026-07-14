//! yukicoder のサンプル取得・コンテスト情報取得。
//!
//! # サンプル取得
//! yukicoder の `/api/v1/problems/{no}/file/in` は `BearerAuth` が必須であり
//! 一般ユーザーが認証なしで呼び出すことはできない。
//! そのため、サンプルは問題ページの HTML をスクレイピングして取得する。
//!
//! HTML 構造:
//! ```html
//! <div class="sample" data-file="01_sample_01.txt">
//!   <h5>サンプル1</h5>
//!   <div class="paragraph">
//!     <h6>入力</h6>
//!     <pre>…入力テキスト…</pre>
//!     <h6>出力</h6>
//!     <pre>…出力テキスト…</pre>
//!   </div>
//! </div>
//! ```
//!
//! # コンテスト情報取得
//! コンテスト情報は REST API を使用する。
//! - コンテスト情報: `GET https://yukicoder.me/api/v1/contest/id/{contest_id}`
//! - 問題情報:       `GET https://yukicoder.me/api/v1/problems/{problem_no}`
//!
//! # URL パターン
//! - コンテスト: `https://yukicoder.me/contests/{contest_id}`
//! - 問題:       `https://yukicoder.me/problems/no/{problem_no}`

use super::model::{ContestMeta, SampleCase, TaskMeta};
use crate::error::AppError;
use scraper::{Html, Selector};
use serde::Deserialize;

const BASE: &str = "https://yukicoder.me";
const API_BASE: &str = "https://yukicoder.me/api/v1";

// ─── URL 判定 ──────────────────────────────────────────────────────

pub fn is_url(url: &str) -> bool {
    url.contains("yukicoder.me")
}

pub fn is_contest_url(url: &str) -> bool {
    is_url(url) && url.contains("/contests/")
}

// ─── API レスポンス型 ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApiContest {
    #[serde(rename = "Name")]
    name: String,
    #[serde(rename = "ProblemIdList")]
    problem_id_list: Vec<u64>,
}

#[derive(Debug, Deserialize)]
struct ApiProblem {
    #[serde(rename = "No")]
    no: u64,
    #[serde(rename = "Title")]
    title: String,
}

// ─── API ヘルパー ──────────────────────────────────────────────────

/// レスポンスのステータスが非成功なら `AppError::ApiError` を返し、
/// 成功なら JSON にデシリアライズして返す。
async fn api_get<T: serde::de::DeserializeOwned>(
    client: &reqwest::Client,
    url: &str,
) -> Result<T, AppError> {
    let resp = client.get(url).send().await?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::ApiError {
            status,
            url: url.to_string(),
            body,
        });
    }
    Ok(resp.json::<T>().await?)
}

// ─── コンテスト取得 ─────────────────────────────────────────────────

pub async fn fetch_contest(url: &str, client: &reqwest::Client) -> Result<ContestMeta, AppError> {
    let contest_id = extract_contest_id(url)?;
    let api_url = format!("{API_BASE}/contest/id/{contest_id}");

    let contest: ApiContest = api_get(client, &api_url).await?;

    // 各問題の情報を取得してタスク一覧を構築
    let mut tasks = Vec::new();
    for problem_id in &contest.problem_id_list {
        let prob_url = format!("{API_BASE}/problems/{problem_id}");
        let problem: ApiProblem = api_get(client, &prob_url).await?;
        tasks.push(TaskMeta {
            id: problem.no.to_string(),
            name: problem.title.clone(),
            url: format!("{BASE}/problems/no/{}", problem.no),
            filename: None,
        });
    }

    Ok(ContestMeta {
        judge: "yukicoder".to_string(),
        contest_id: contest_id.clone(),
        contest_name: contest.name,
        url: url.to_string(),
        tasks,
    })
}

// ─── サンプル取得 ───────────────────────────────────────────────────

/// 問題ページの HTML をスクレイピングしてサンプルケースを取得する。
///
/// yukicoder の `/api/v1/problems/{no}/file/{in,out}` は `BearerAuth` が必須で
/// 認証なしでは利用できないため、HTML から `div.sample` を解析する方式を採用する。
///
/// # HTML 構造
/// ```html
/// <div class="sample" data-file="01_sample_01.txt">
///   <div class="paragraph">
///     <h6>入力</h6><pre>…</pre>
///     <h6>出力</h6><pre>…</pre>
///   </div>
/// </div>
/// ```
pub async fn fetch_samples(
    url: &str,
    client: &reqwest::Client,
) -> Result<Vec<SampleCase>, AppError> {
    let problem_no = extract_problem_no(url)?;
    let page_url = format!("{BASE}/problems/no/{problem_no}");

    let resp = client.get(&page_url).send().await?;
    if !resp.status().is_success() {
        let status = resp.status().as_u16();
        let body = resp.text().await.unwrap_or_default();
        return Err(AppError::ApiError {
            status,
            url: page_url,
            body,
        });
    }
    let html = resp.text().await?;

    parse_samples(&html).map_err(AppError::SampleParse)
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
fn inner_text(el: scraper::ElementRef) -> String {
    use scraper::node::Node;

    /// ブロック要素として扱うタグ名（内容の後に \n を挿入する）。
    const BLOCK_TAGS: &[&str] = &[
        "div", "p", "li", "tr", "td", "th", "h1", "h2", "h3", "h4", "h5", "h6",
    ];

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

/// HTML 文字列から `div.sample` ブロックを解析してサンプルケースを返す。
///
/// 各 `div.sample` ブロック内の `pre` タグを順に取得し、
/// 偶数番目（0-indexed）を入力、奇数番目を出力として対にする。
fn parse_samples(html: &str) -> Result<Vec<SampleCase>, String> {
    let document = Html::parse_document(html);

    let sample_sel = Selector::parse("div.sample").map_err(|e| e.to_string())?;
    let pre_sel = Selector::parse("pre").map_err(|e| e.to_string())?;

    let mut samples = Vec::new();

    for sample_div in document.select(&sample_sel) {
        let pres: Vec<String> = sample_div.select(&pre_sel).map(inner_text).collect();

        // 各 div.sample に <pre>入力</pre> <pre>出力</pre> が含まれる
        if pres.len() >= 2 {
            samples.push(SampleCase {
                input: pres[0].clone(),
                output: pres[1].clone(),
            });
        }
    }

    if samples.is_empty() {
        return Err("no sample blocks (div.sample) found in the page".to_string());
    }

    Ok(samples)
}

// ─── ヘルパー ──────────────────────────────────────────────────────

fn extract_contest_id(url: &str) -> Result<String, AppError> {
    // https://yukicoder.me/contests/1234 → "1234"
    url.trim_end_matches('/')
        .split("/contests/")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::UnsupportedUrl(url.to_string()))
}

fn extract_problem_no(url: &str) -> Result<String, AppError> {
    // https://yukicoder.me/problems/no/1 → "1"
    url.trim_end_matches('/')
        .split("/problems/no/")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::UnsupportedUrl(url.to_string()))
}

/// yukicoder API を利用してコンテスト一覧を取得する。
pub async fn fetch_contest_list(
    client: &reqwest::Client,
) -> Result<Vec<super::model::SimpleContest>, AppError> {
    let url = "https://yukicoder.me/api/v1/contest/past";

    #[derive(Debug, serde::Deserialize)]
    struct YukiContest {
        #[serde(rename = "Id")]
        id: u64,
        #[serde(rename = "Name")]
        name: String,
    }

    let resp: Vec<YukiContest> = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "je-cli")
        .send()
        .await?
        .json()
        .await
        .map_err(|e| AppError::SampleParse(format!("Failed to fetch yukicoder contests: {}", e)))?;

    let mut contests = resp
        .into_iter()
        .map(|c| super::model::SimpleContest {
            id: c.id.to_string(),
            name: c.name,
            url: format!("https://yukicoder.me/contests/{}", c.id),
        })
        .collect::<Vec<_>>();

    // IDの降順（最新順）にソート
    contests.sort_by(|a, b| {
        let a_id: u64 = a.id.parse().unwrap_or(0);
        let b_id: u64 = b.id.parse().unwrap_or(0);
        b_id.cmp(&a_id)
    });

    Ok(contests)
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── URL 判定 ─────────────────────────────────────────────────

    #[test]
    fn is_url_contest() {
        assert!(is_url("https://yukicoder.me/contests/400"));
    }

    #[test]
    fn is_url_problem() {
        assert!(is_url("https://yukicoder.me/problems/no/1"));
    }

    #[test]
    fn is_url_false_for_other() {
        assert!(!is_url("https://atcoder.jp/contests/abc001"));
    }

    #[test]
    fn is_contest_url_true() {
        assert!(is_contest_url("https://yukicoder.me/contests/400"));
    }

    #[test]
    fn is_contest_url_false_for_problem() {
        assert!(!is_contest_url("https://yukicoder.me/problems/no/1"));
    }

    // ─── extract_contest_id ──────────────────────────────────────

    #[test]
    fn extract_contest_id_basic() {
        let id = extract_contest_id("https://yukicoder.me/contests/400").unwrap();
        assert_eq!(id, "400");
    }

    #[test]
    fn extract_contest_id_trailing_slash() {
        let id = extract_contest_id("https://yukicoder.me/contests/400/").unwrap();
        assert_eq!(id, "400");
    }

    #[test]
    fn extract_contest_id_unsupported() {
        let err = extract_contest_id("https://example.com/foo").unwrap_err();
        assert!(matches!(err, AppError::UnsupportedUrl(_)));
    }

    // ─── extract_problem_no ──────────────────────────────────────

    #[test]
    fn extract_problem_no_basic() {
        let no = extract_problem_no("https://yukicoder.me/problems/no/1").unwrap();
        assert_eq!(no, "1");
    }

    #[test]
    fn extract_problem_no_large_number() {
        let no = extract_problem_no("https://yukicoder.me/problems/no/9999").unwrap();
        assert_eq!(no, "9999");
    }

    #[test]
    fn extract_problem_no_trailing_slash() {
        let no = extract_problem_no("https://yukicoder.me/problems/no/42/").unwrap();
        assert_eq!(no, "42");
    }

    #[test]
    fn extract_problem_no_unsupported() {
        let err = extract_problem_no("https://example.com/foo").unwrap_err();
        assert!(matches!(err, AppError::UnsupportedUrl(_)));
    }

    // ─── parse_samples ───────────────────────────────────────────

    #[test]
    fn parse_samples_single() {
        let html = r#"
            <html><body>
            <div class="sample" data-file="01_sample_01.txt">
              <h5>サンプル1</h5>
              <div class="paragraph">
                <h6>入力</h6>
                <pre>3
100
</pre>
                <h6>出力</h6>
                <pre>20
</pre>
              </div>
            </div>
            </body></html>
        "#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].input, "3\n100\n");
        assert_eq!(samples[0].output, "20\n");
    }

    #[test]
    fn parse_samples_multiple() {
        let html = r#"
            <html><body>
            <div class="sample" data-file="01_sample_01.txt">
              <div class="paragraph">
                <pre>1</pre>
                <pre>YES</pre>
              </div>
            </div>
            <div class="sample" data-file="01_sample_02.txt">
              <div class="paragraph">
                <pre>0</pre>
                <pre>NO</pre>
              </div>
            </div>
            </body></html>
        "#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].input, "1");
        assert_eq!(samples[0].output, "YES");
        assert_eq!(samples[1].input, "0");
        assert_eq!(samples[1].output, "NO");
    }

    #[test]
    fn parse_samples_br_newlines() {
        // <pre> 内の改行が <br> で表現されている場合でも正しく複数行として取得できること。
        let html = r#"
            <html><body>
            <div class="sample" data-file="01_sample_01.txt">
              <div class="paragraph">
                <pre>3<br/>1 2 3</pre>
                <pre>6</pre>
              </div>
            </div>
            </body></html>
        "#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].input, "3\n1 2 3");
        assert_eq!(samples[0].output, "6");
    }

    #[test]
    fn parse_samples_no_sample_div_returns_err() {
        let html = "<html><body><p>no samples here</p></body></html>";
        let err = parse_samples(html).unwrap_err();
        assert!(err.contains("no sample blocks"));
    }

    #[test]
    fn parse_samples_skips_incomplete_block() {
        // pre が 1 つしかないブロックはスキップ、2つあるものだけ返す
        let html = r#"
            <html><body>
            <div class="sample"><pre>only input</pre></div>
            <div class="sample"><pre>input</pre><pre>output</pre></div>
            </body></html>
        "#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].input, "input");
        assert_eq!(samples[0].output, "output");
    }
}
