//! AtCoder のサンプル取得・コンテスト情報取得。
//!
//! # URL パターン
//! ## 現行形式
//! - コンテスト: `https://atcoder.jp/contests/{contest_id}`
//! - 問題:       `https://atcoder.jp/contests/{contest_id}/tasks/{task_id}`
//!
//! ## 旧形式（サブドメイン形式）
//! - コンテスト: `https://{contest_id}.contest.atcoder.jp/`
//! - 問題:       `https://{contest_id}.contest.atcoder.jp/tasks/{task_id}`
//!
//! 旧形式 URL は [`normalize_url`] で現行形式に変換してから処理する。

use super::model::{ContestMeta, SampleCase, TaskMeta};
use crate::error::AppError;
use scraper::{Html, Selector};
use std::time::Duration;
use tokio::time::sleep;

/// AtCoder へのリクエスト間の待機時間（過負荷防止）。
const REQUEST_INTERVAL: Duration = Duration::from_secs(1);

const BASE: &str = "https://atcoder.jp";

// ─── URL 判定 ──────────────────────────────────────────────────────

/// AtCoder のコンテスト URL か問題 URL のいずれかであれば `true`。
///
/// 現行形式 (`atcoder.jp/contests/`) と旧サブドメイン形式
/// (`*.contest.atcoder.jp`) の両方を認識する。
pub fn is_url(url: &str) -> bool {
    url.contains("atcoder.jp/contests/") || is_legacy_url(url)
}

/// 問題 URL なら `true`。
///
/// `/tasks/` の後ろにタスク ID が続く場合のみ問題 URL と判定する。
/// `/contests/abc001/tasks`（一覧ページ）は問題 URL ではない。
///
/// 例 (現行): `https://atcoder.jp/contests/abc001/tasks/abc001_a`
/// 例 (旧形式): `https://abc001.contest.atcoder.jp/tasks/abc001_a`
pub fn is_problem_url(url: &str) -> bool {
    is_url(url) && url.contains("/tasks/")
}

/// コンテスト URL（タスク URL ではない）なら `true`。
///
/// 問題 URL（`/tasks/` の後ろにタスク ID あり）でなければコンテスト URL と見なす。
/// これにより `/contests/abc001/tasks`（タスク一覧）もコンテスト URL として扱われる。
///
/// 例 (現行): `https://atcoder.jp/contests/abc001`
/// 例 (一覧): `https://atcoder.jp/contests/abc001/tasks`  ← コンテスト URL として扱う
/// 例 (旧形式): `https://abc001.contest.atcoder.jp/`
pub fn is_contest_url(url: &str) -> bool {
    is_url(url) && !is_problem_url(url)
}

/// 旧サブドメイン形式 (`{id}.contest.atcoder.jp`) の URL なら `true`。
fn is_legacy_url(url: &str) -> bool {
    url.contains(".contest.atcoder.jp")
}

/// 旧形式 URL を現行形式 URL へ正規化する。
///
/// - `https://abc001.contest.atcoder.jp/`
///   → `https://atcoder.jp/contests/abc001`
/// - `https://abc001.contest.atcoder.jp/tasks/abc001_a`
///   → `https://atcoder.jp/contests/abc001/tasks/abc001_a`
///
/// 旧形式でない URL はそのまま返す。
pub fn normalize_url(url: &str) -> String {
    if !is_legacy_url(url) {
        return url.to_string();
    }

    let contest_id = extract_contest_id_legacy(url).unwrap_or_default();

    // パス部分 ("/tasks/abc001_a" など) を取り出す
    // スキーム + ホスト以降のパス: "https://abc001.contest.atcoder.jp/tasks/abc001_a"
    //   → "/tasks/abc001_a"
    let path = url
        .split_once("://")
        .and_then(|(_, rest)| rest.split_once('/'))
        .map(|(_, path)| path.trim_end_matches('/'))
        .unwrap_or("");

    if path.is_empty() {
        format!("{BASE}/contests/{contest_id}")
    } else {
        format!("{BASE}/contests/{contest_id}/{path}")
    }
}

/// 旧形式 URL (`{id}.contest.atcoder.jp`) からコンテスト ID を抽出する。
fn extract_contest_id_legacy(url: &str) -> Result<String, AppError> {
    // "https://abc001.contest.atcoder.jp/..." → "abc001"
    url.split_once("://")
        .map(|(_, rest)| rest)
        .and_then(|rest| rest.split('.').next())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::UnsupportedUrl(url.to_string()))
}

// ─── コンテスト取得 ─────────────────────────────────────────────────

/// コンテスト URL からタスク一覧を含むメタデータを取得する。
///
/// 旧サブドメイン形式 URL が渡された場合は自動的に現行形式へ正規化する。
pub async fn fetch_contest(
    url: &str,
    client: &reqwest::Client,
) -> Result<ContestMeta, AppError> {
    let url = &normalize_url(url);

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
///
/// 旧サブドメイン形式 URL が渡された場合は自動的に現行形式へ正規化する。
pub async fn fetch_samples(
    url: &str,
    client: &reqwest::Client,
) -> Result<Vec<SampleCase>, AppError> {
    let url = normalize_url(url);
    let html = fetch_html(&url, client).await?;
    parse_samples(&html)
}

// ─── パース ────────────────────────────────────────────────────────

/// タスク一覧テーブルをパースして `Vec<TaskMeta>` を返す。
///
/// AtCoder のタスクテーブルは `table.table tbody` に含まれており、
/// 各行の構造は以下のとおり：
///
/// ```html
/// <tr>
///   <td class="text-center no-break"><a href="/contests/{id}/tasks/{task_id}">A</a></td>
///   <td><a href="/contests/{id}/tasks/{task_id}">Task Name</a></td>
///   <td>2 sec</td>
///   <td>1024 MiB</td>
/// </tr>
/// ```
///
/// 1 列目の `<a>` からアルファベット ID と URL、2 列目の `<a>` からタスク名を取得する。
fn parse_task_table(html: &str, contest_id: &str) -> Result<Vec<TaskMeta>, AppError> {
    let doc = Html::parse_document(html);
    let row_sel = Selector::parse("table.table tbody tr").unwrap();
    let td_sel = Selector::parse("td").unwrap();
    let a_sel = Selector::parse("a").unwrap();

    let mut tasks = Vec::new();

    for row in doc.select(&row_sel) {
        let cols: Vec<_> = row.select(&td_sel).collect();
        if cols.len() < 2 {
            continue;
        }

        // 1 列目: <a>A</a> — アルファベット ID と問題 URL
        let id_cell = &cols[0];
        let id_anchor = id_cell.select(&a_sel).next();
        let id = id_anchor
            .map(|a| a.text().collect::<String>().trim().to_lowercase())
            .unwrap_or_else(|| id_cell.text().collect::<String>().trim().to_lowercase());
        let href = id_anchor
            .and_then(|a| a.value().attr("href"))
            // フォールバック: 2 列目の <a> から URL を取る
            .or_else(|| {
                cols[1]
                    .select(&a_sel)
                    .next()
                    .and_then(|a| a.value().attr("href"))
            })
            .unwrap_or("");

        // 2 列目: タスク名
        let name = cols[1].text().collect::<String>().trim().to_string();

        if id.is_empty() || href.is_empty() {
            continue;
        }

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
///
/// # HTML 構造（新形式: abc188 以降）
/// ```html
/// <section>
///   <h3>入力例 1</h3>
///   <pre>3 5</pre>
/// </section>
/// <section>
///   <h3>出力例 1</h3>
///   <pre>8</pre>
/// </section>
/// ```
///
/// # HTML 構造（旧形式: abc001 前後）
/// ```html
/// <div class="part">
///   <h3>入力例 1</h3>
///   <section>
///     <pre class="prettyprint linenums">3 5</pre>
///   </section>
/// </div>
/// ```
///
/// `<pre><code>...</code></pre>` のネスト構造にも対応する。
fn parse_samples(html: &str) -> Result<Vec<SampleCase>, AppError> {
    let doc = Html::parse_document(html);
    let section_sel = Selector::parse("section").unwrap();
    let h3_sel = Selector::parse("h3").unwrap();
    let pre_sel = Selector::parse("pre").unwrap();

    let mut inputs: Vec<String> = Vec::new();
    let mut outputs: Vec<String> = Vec::new();

    // ── 新形式: <section> の中に <h3> と <pre> が同居 ──────────────
    for section in doc.select(&section_sel) {
        let heading = section
            .select(&h3_sel)
            .next()
            .map(|h| h.text().collect::<String>())
            .unwrap_or_default();

        // <pre> 内に <code> がネストされていても text() はすべての子孫テキストを連結する
        let pre_text = section
            .select(&pre_sel)
            .next()
            .map(|p| normalize_pre_text(p.text().collect::<String>()))
            .unwrap_or_default();

        if heading.contains("入力例") || heading.contains("Sample Input") {
            inputs.push(pre_text);
        } else if heading.contains("出力例") || heading.contains("Sample Output") {
            outputs.push(pre_text);
        }
    }

    // ── 旧形式フォールバック: <div class="part"> 直下に <h3>、兄弟 <section> に <pre> ──
    if inputs.is_empty() {
        let part_sel = Selector::parse("div.part").unwrap();

        for part in doc.select(&part_sel) {
            // <div class="part"> の直接の子 <h3> からヘッダーを取得
            let heading = part
                .select(&h3_sel)
                .next()
                .map(|h| h.text().collect::<String>())
                .unwrap_or_default();

            // <div class="part"> 配下の <section> 内 <pre> からコンテンツを取得
            let pre_text = part
                .select(&pre_sel)
                .next()
                .map(|p| normalize_pre_text(p.text().collect::<String>()))
                .unwrap_or_default();

            if heading.contains("入力例") || heading.contains("Sample Input") {
                inputs.push(pre_text);
            } else if heading.contains("出力例") || heading.contains("Sample Output") {
                outputs.push(pre_text);
            }
        }
    }

    if inputs.is_empty() {
        return Err(AppError::SampleParse(
            "No sample inputs found on this page".to_string(),
        ));
    }

    if inputs.len() != outputs.len() {
        return Err(AppError::SampleParse(format!(
            "Sample input/output count mismatch: {} input(s) vs {} output(s)",
            inputs.len(),
            outputs.len(),
        )));
    }

    let samples = inputs
        .into_iter()
        .zip(outputs)
        .map(|(input, output)| SampleCase { input, output })
        .collect();

    Ok(samples)
}

/// `<pre>` テキストの末尾改行を統一する。
///
/// AtCoder の `<pre>` ブロックは末尾に `\n` が付くことが多い。
/// ここでは末尾の空白を取り除いたうえで `\n` を 1 つ付加し、
/// 後続の比較処理で扱いやすい形に正規化する。
fn normalize_pre_text(s: String) -> String {
    let trimmed = s.trim_end_matches(['\n', '\r', ' ']);
    if trimmed.is_empty() {
        String::new()
    } else {
        format!("{trimmed}\n")
    }
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
    if is_legacy_url(url) {
        return extract_contest_id_legacy(url);
    }
    // "https://atcoder.jp/contests/abc001" → "abc001"
    url.trim_end_matches('/')
        .split("/contests/")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::UnsupportedUrl(url.to_string()))
}

/// URL から HTML を取得する。
///
/// リクエスト前に [`REQUEST_INTERVAL`] だけ待機し、AtCoder サーバーへの
/// 過負荷を防ぐ。
async fn fetch_html(url: &str, client: &reqwest::Client) -> Result<String, AppError> {
    sleep(REQUEST_INTERVAL).await;
    let resp = client.get(url).send().await?;
    Ok(resp.text().await?)
}

/// AtCoder Problems API を利用してコンテスト一覧を取得する。
pub async fn fetch_contest_list(
    client: &reqwest::Client,
) -> Result<Vec<super::model::SimpleContest>, AppError> {
    let url = "https://kenkoooo.com/atcoder/resources/contests.json";
    let resp: Vec<serde_json::Value> = client
        .get(url)
        .header(reqwest::header::USER_AGENT, "je-cli")
        .send()
        .await?
        .json()
        .await
        .map_err(|e| AppError::SampleParse(format!("Failed to fetch AtCoder contests: {}", e)))?;

    let mut contests: Vec<(i64, super::model::SimpleContest)> = Vec::new();
    for item in resp {
        if let (Some(id), Some(title)) = (
            item.get("id").and_then(|v| v.as_str()),
            item.get("title").and_then(|v| v.as_str()),
        ) {
            let epoch = item
                .get("start_epoch_second")
                .and_then(|v| v.as_i64())
                .unwrap_or(0);
            contests.push((epoch, super::model::SimpleContest {
                id: id.to_string(),
                name: title.to_string(),
                url: format!("https://atcoder.jp/contests/{}", id),
            }));
        }
    }

    // start_epoch_second の降順（最新のコンテストが先頭）
    contests.sort_by_key(|c| std::cmp::Reverse(c.0));
    Ok(contests.into_iter().map(|(_, c)| c).collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── URL 判定（現行形式） ─────────────────────────────────────

    #[test]
    fn is_url_contest() {
        assert!(is_url("https://atcoder.jp/contests/abc001"));
    }

    #[test]
    fn is_url_problem() {
        assert!(is_url(
            "https://atcoder.jp/contests/abc001/tasks/abc001_a"
        ));
    }

    #[test]
    fn is_url_false_for_other_site() {
        assert!(!is_url("https://codeforces.com/contest/1234"));
    }

    #[test]
    fn is_contest_url_true() {
        assert!(is_contest_url("https://atcoder.jp/contests/abc001"));
    }

    #[test]
    fn is_contest_url_false_for_problem() {
        assert!(!is_contest_url(
            "https://atcoder.jp/contests/abc001/tasks/abc001_a"
        ));
    }

    #[test]
    fn is_contest_url_true_for_tasks_list() {
        // /tasks 末尾（タスク一覧ページ）はコンテスト URL として扱う
        assert!(is_contest_url(
            "https://atcoder.jp/contests/abc001/tasks"
        ));
    }

    #[test]
    fn is_problem_url_true() {
        assert!(is_problem_url(
            "https://atcoder.jp/contests/abc001/tasks/abc001_a"
        ));
    }

    #[test]
    fn is_problem_url_false_for_contest() {
        assert!(!is_problem_url("https://atcoder.jp/contests/abc001"));
    }

    #[test]
    fn is_problem_url_false_for_tasks_list() {
        // /tasks 末尾はタスク一覧ページなので問題 URL ではない
        assert!(!is_problem_url(
            "https://atcoder.jp/contests/abc001/tasks"
        ));
    }

    // ─── URL 判定（旧サブドメイン形式） ──────────────────────────

    #[test]
    fn is_url_legacy_contest() {
        assert!(is_url("https://abc001.contest.atcoder.jp/"));
    }

    #[test]
    fn is_url_legacy_contest_http() {
        assert!(is_url("http://abc001.contest.atcoder.jp/"));
    }

    #[test]
    fn is_url_legacy_problem() {
        assert!(is_url(
            "https://abc001.contest.atcoder.jp/tasks/abc001_a"
        ));
    }

    #[test]
    fn is_contest_url_legacy_true() {
        assert!(is_contest_url("https://abc001.contest.atcoder.jp/"));
    }

    #[test]
    fn is_contest_url_legacy_false_for_problem() {
        assert!(!is_contest_url(
            "https://abc001.contest.atcoder.jp/tasks/abc001_a"
        ));
    }

    #[test]
    fn is_problem_url_legacy_true() {
        assert!(is_problem_url(
            "https://abc001.contest.atcoder.jp/tasks/abc001_a"
        ));
    }

    #[test]
    fn is_problem_url_legacy_false_for_contest() {
        assert!(!is_problem_url("https://abc001.contest.atcoder.jp/"));
    }

    // ─── normalize_url ───────────────────────────────────────────

    #[test]
    fn normalize_url_legacy_contest() {
        assert_eq!(
            normalize_url("https://abc001.contest.atcoder.jp/"),
            "https://atcoder.jp/contests/abc001"
        );
    }

    #[test]
    fn normalize_url_legacy_contest_no_trailing_slash() {
        assert_eq!(
            normalize_url("https://abc001.contest.atcoder.jp"),
            "https://atcoder.jp/contests/abc001"
        );
    }

    #[test]
    fn normalize_url_legacy_problem() {
        assert_eq!(
            normalize_url("https://abc001.contest.atcoder.jp/tasks/abc001_a"),
            "https://atcoder.jp/contests/abc001/tasks/abc001_a"
        );
    }

    #[test]
    fn normalize_url_legacy_http_scheme() {
        assert_eq!(
            normalize_url("http://abc001.contest.atcoder.jp/tasks/abc001_a"),
            "https://atcoder.jp/contests/abc001/tasks/abc001_a"
        );
    }

    #[test]
    fn normalize_url_modern_is_unchanged() {
        let url = "https://atcoder.jp/contests/abc001/tasks/abc001_a";
        assert_eq!(normalize_url(url), url);
    }

    // ─── extract_contest_id ──────────────────────────────────────

    #[test]
    fn extract_contest_id_simple() {
        let id = extract_contest_id("https://atcoder.jp/contests/abc001").unwrap();
        assert_eq!(id, "abc001");
    }

    #[test]
    fn extract_contest_id_trailing_slash() {
        let id = extract_contest_id("https://atcoder.jp/contests/abc001/").unwrap();
        assert_eq!(id, "abc001");
    }

    #[test]
    fn extract_contest_id_from_problem_url() {
        let id = extract_contest_id(
            "https://atcoder.jp/contests/abc001/tasks/abc001_a",
        )
        .unwrap();
        assert_eq!(id, "abc001");
    }

    #[test]
    fn extract_contest_id_unsupported_url() {
        let err = extract_contest_id("https://example.com/foo").unwrap_err();
        assert!(matches!(err, AppError::UnsupportedUrl(_)));
    }

    #[test]
    fn extract_contest_id_legacy_contest() {
        let id = extract_contest_id("https://abc001.contest.atcoder.jp/").unwrap();
        assert_eq!(id, "abc001");
    }

    #[test]
    fn extract_contest_id_legacy_problem() {
        let id = extract_contest_id(
            "https://abc001.contest.atcoder.jp/tasks/abc001_a",
        )
        .unwrap();
        assert_eq!(id, "abc001");
    }

    // ─── parse_contest_name ──────────────────────────────────────

    #[test]
    fn parse_contest_name_basic() {
        let html = "<html><head><title>AtCoder Beginner Contest 001 - AtCoder</title></head><body></body></html>";
        let name = parse_contest_name(html).unwrap();
        assert_eq!(name, "AtCoder Beginner Contest 001");
    }

    #[test]
    fn parse_contest_name_no_title() {
        let html = "<html><head></head><body></body></html>";
        assert!(parse_contest_name(html).is_none());
    }

    #[test]
    fn parse_contest_name_empty_title() {
        let html = "<html><head><title></title></head></html>";
        assert!(parse_contest_name(html).is_none());
    }

    // ─── parse_task_table ────────────────────────────────────────

    #[test]
    fn parse_task_table_basic() {
        // 実際の AtCoder HTML 構造: 1 列目の <a> に ID と URL、2 列目にタスク名
        let html = r#"
<table class="table table-bordered table-striped">
  <tbody>
    <tr>
      <td class="text-center no-break"><a href="/contests/abc001/tasks/abc001_a">A</a></td>
      <td><a href="/contests/abc001/tasks/abc001_a">Two Sum</a></td>
      <td class="text-right">2 sec</td>
      <td class="text-right">1024 MiB</td>
    </tr>
    <tr>
      <td class="text-center no-break"><a href="/contests/abc001/tasks/abc001_b">B</a></td>
      <td><a href="/contests/abc001/tasks/abc001_b">Difference</a></td>
      <td class="text-right">2 sec</td>
      <td class="text-right">1024 MiB</td>
    </tr>
  </tbody>
</table>
"#;
        let tasks = parse_task_table(html, "abc001").unwrap();
        assert_eq!(tasks.len(), 2);
        assert_eq!(tasks[0].id, "a");
        assert_eq!(tasks[0].name, "Two Sum");
        assert!(tasks[0].url.contains("abc001_a"));
        assert_eq!(tasks[1].id, "b");
        assert_eq!(tasks[1].name, "Difference");
    }

    #[test]
    fn parse_task_table_absolute_href() {
        let html = r#"
<table class="table table-bordered table-striped">
  <tbody>
    <tr>
      <td class="text-center no-break"><a href="https://atcoder.jp/contests/abc001/tasks/abc001_a">A</a></td>
      <td><a href="https://atcoder.jp/contests/abc001/tasks/abc001_a">A problem</a></td>
      <td class="text-right">2 sec</td>
      <td class="text-right">1024 MiB</td>
    </tr>
  </tbody>
</table>
"#;
        let tasks = parse_task_table(html, "abc001").unwrap();
        assert_eq!(tasks[0].url, "https://atcoder.jp/contests/abc001/tasks/abc001_a");
    }

    #[test]
    fn parse_task_table_empty_returns_error() {
        let html = "<html><body></body></html>";
        let err = parse_task_table(html, "abc001").unwrap_err();
        assert!(matches!(err, AppError::SampleParse(_)));
    }

    // ─── parse_samples ───────────────────────────────────────────

    #[test]
    fn parse_samples_japanese_labels() {
        let html = r#"
<html><body>
  <section>
    <h3>入力例 1</h3>
    <pre>3 5</pre>
  </section>
  <section>
    <h3>出力例 1</h3>
    <pre>8</pre>
  </section>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].input.trim(), "3 5");
        assert_eq!(samples[0].output.trim(), "8");
    }

    #[test]
    fn parse_samples_english_labels() {
        let html = r#"
<html><body>
  <section>
    <h3>Sample Input 1</h3>
    <pre>1 2 3</pre>
  </section>
  <section>
    <h3>Sample Output 1</h3>
    <pre>6</pre>
  </section>
  <section>
    <h3>Sample Input 2</h3>
    <pre>10 20</pre>
  </section>
  <section>
    <h3>Sample Output 2</h3>
    <pre>30</pre>
  </section>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[1].input.trim(), "10 20");
        assert_eq!(samples[1].output.trim(), "30");
    }

    #[test]
    fn parse_samples_no_samples_returns_error() {
        let html = "<html><body><p>Nothing here</p></body></html>";
        let err = parse_samples(html).unwrap_err();
        assert!(matches!(err, AppError::SampleParse(_)));
    }

    #[test]
    fn parse_samples_pre_code_nested() {
        // AtCoder の一部ページでは <pre><code>...</code></pre> 構造を持つ
        let html = r#"
<html><body>
  <section>
    <h3>入力例 1</h3>
    <pre><code>3 5
</code></pre>
  </section>
  <section>
    <h3>出力例 1</h3>
    <pre><code>8
</code></pre>
  </section>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].input, "3 5\n");
        assert_eq!(samples[0].output, "8\n");
    }

    #[test]
    fn parse_samples_normalizes_trailing_newline() {
        // <pre> テキストが末尾に複数の改行や空白を持つ場合でも \n 1 つに正規化される
        let html = r#"
<html><body>
  <section>
    <h3>Sample Input 1</h3>
    <pre>1 2 3


</pre>
  </section>
  <section>
    <h3>Sample Output 1</h3>
    <pre>6   </pre>
  </section>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples[0].input, "1 2 3\n");
        assert_eq!(samples[0].output, "6\n");
    }

    #[test]
    fn parse_samples_input_output_count_mismatch_returns_error() {
        // 出力例が入力例より少ない場合はエラー
        let html = r#"
<html><body>
  <section>
    <h3>入力例 1</h3>
    <pre>3 5</pre>
  </section>
  <section>
    <h3>入力例 2</h3>
    <pre>10 20</pre>
  </section>
  <section>
    <h3>出力例 1</h3>
    <pre>8</pre>
  </section>
</body></html>
"#;
        let err = parse_samples(html).unwrap_err();
        assert!(matches!(err, AppError::SampleParse(_)));
        let msg = err.to_string();
        assert!(msg.contains("2") && msg.contains("1"), "エラーメッセージに件数が含まれること: {msg}");
    }

    #[test]
    fn parse_samples_multiple_japanese() {
        // 日本語ラベルで複数サンプル
        let html = r#"
<html><body>
  <section><h3>入力例 1</h3><pre>1</pre></section>
  <section><h3>出力例 1</h3><pre>2</pre></section>
  <section><h3>入力例 2</h3><pre>3</pre></section>
  <section><h3>出力例 2</h3><pre>4</pre></section>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].input.trim(), "1");
        assert_eq!(samples[0].output.trim(), "2");
        assert_eq!(samples[1].input.trim(), "3");
        assert_eq!(samples[1].output.trim(), "4");
    }

    // ─── 旧形式 (div.part) ─────────────────────────────────────

    #[test]
    fn parse_samples_legacy_div_part_structure() {
        // abc001 など旧ページ: <h3> が <div class="part"> の直下にあり、
        // <section> の兄弟になっている構造
        let html = r#"
<html><body>
  <div class="part">
    <h3>入力例 1</h3>
    <section>
      <pre class="prettyprint linenums">15
10</pre>
    </section>
  </div>
  <div class="part">
    <h3>出力例 1</h3>
    <section>
      <pre class="prettyprint linenums">1</pre>
    </section>
  </div>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].input, "15\n10\n");
        assert_eq!(samples[0].output, "1\n");
    }

    #[test]
    fn parse_samples_legacy_multiple_cases() {
        // 旧形式で複数サンプル
        let html = r#"
<html><body>
  <div class="part">
    <h3>入力例 1</h3>
    <section><pre class="prettyprint linenums">3 5</pre></section>
  </div>
  <div class="part">
    <h3>出力例 1</h3>
    <section><pre class="prettyprint linenums">8</pre></section>
  </div>
  <div class="part">
    <h3>入力例 2</h3>
    <section><pre class="prettyprint linenums">10 20</pre></section>
  </div>
  <div class="part">
    <h3>出力例 2</h3>
    <section><pre class="prettyprint linenums">30</pre></section>
  </div>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 2);
        assert_eq!(samples[0].input.trim(), "3 5");
        assert_eq!(samples[0].output.trim(), "8");
        assert_eq!(samples[1].input.trim(), "10 20");
        assert_eq!(samples[1].output.trim(), "30");
    }

    #[test]
    fn parse_samples_legacy_english_labels() {
        // 旧形式で英語ラベル
        let html = r#"
<html><body>
  <div class="part">
    <h3>Sample Input 1</h3>
    <section><pre class="prettyprint linenums">1 2 3</pre></section>
  </div>
  <div class="part">
    <h3>Sample Output 1</h3>
    <section><pre class="prettyprint linenums">6</pre></section>
  </div>
</body></html>
"#;
        let samples = parse_samples(html).unwrap();
        assert_eq!(samples.len(), 1);
        assert_eq!(samples[0].input.trim(), "1 2 3");
        assert_eq!(samples[0].output.trim(), "6");
    }

    // ─── normalize_pre_text ─────────────────────────────────────

    #[test]
    fn normalize_pre_text_strips_trailing_newlines() {
        assert_eq!(normalize_pre_text("3 5\n\n".to_string()), "3 5\n");
    }

    #[test]
    fn normalize_pre_text_strips_trailing_spaces() {
        assert_eq!(normalize_pre_text("8   ".to_string()), "8\n");
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
}
