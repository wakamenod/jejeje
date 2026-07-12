//! yukicoder のサンプル取得・コンテスト情報取得。
//!
//! yukicoder は公式 REST API を提供しているため、スクレイピングではなく API を使用する。
//!
//! # API エンドポイント
//! - コンテスト情報: `GET https://yukicoder.me/api/v1/contest/id/{contest_id}`
//! - 問題情報:       `GET https://yukicoder.me/api/v1/problems/{problem_no}`
//!
//! # URL パターン
//! - コンテスト: `https://yukicoder.me/contests/{contest_id}`
//! - 問題:       `https://yukicoder.me/problems/no/{problem_no}`

use super::model::{ContestMeta, SampleCase, TaskMeta};
use crate::error::AppError;
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

pub fn is_problem_url(url: &str) -> bool {
    is_url(url) && url.contains("/problems/no/")
}

// ─── API レスポンス型 ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApiContest {
    #[serde(rename = "Id")]
    id: u64,
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

#[derive(Debug, Deserialize)]
struct ApiSample {
    #[serde(rename = "Input")]
    input: String,
    #[serde(rename = "Output")]
    output: String,
}

// ─── コンテスト取得 ─────────────────────────────────────────────────

pub async fn fetch_contest(
    url: &str,
    client: &reqwest::Client,
) -> Result<ContestMeta, AppError> {
    let contest_id = extract_contest_id(url)?;
    let api_url = format!("{API_BASE}/contest/id/{contest_id}");

    let contest: ApiContest = client
        .get(&api_url)
        .send()
        .await?
        .json()
        .await?;

    // 各問題の情報を取得してタスク一覧を構築
    let mut tasks = Vec::new();
    for problem_id in &contest.problem_id_list {
        let prob_url = format!("{API_BASE}/problems/{problem_id}");
        if let Ok(problem) = client
            .get(&prob_url)
            .send()
            .await?
            .json::<ApiProblem>()
            .await
        {
            tasks.push(TaskMeta {
                id: problem.no.to_string(),
                name: problem.title.clone(),
                url: format!("{BASE}/problems/no/{}", problem.no),
            });
        }
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

pub async fn fetch_samples(
    url: &str,
    client: &reqwest::Client,
) -> Result<Vec<SampleCase>, AppError> {
    let problem_no = extract_problem_no(url)?;
    let api_url = format!("{API_BASE}/problems/{problem_no}/file/in");

    // yukicoder API でサンプル一覧を取得
    // GET /api/v1/problems/{no}/file/in → サンプル入出力の配列
    let samples: Vec<ApiSample> = client
        .get(&api_url)
        .send()
        .await?
        .json()
        .await
        .map_err(|_| AppError::SampleParse("Failed to fetch samples from yukicoder API".to_string()))?;

    Ok(samples
        .into_iter()
        .map(|s| SampleCase {
            input: s.input,
            output: s.output,
        })
        .collect())
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

    #[test]
    fn is_problem_url_true() {
        assert!(is_problem_url("https://yukicoder.me/problems/no/1"));
    }

    #[test]
    fn is_problem_url_false_for_contest() {
        assert!(!is_problem_url("https://yukicoder.me/contests/400"));
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
}
