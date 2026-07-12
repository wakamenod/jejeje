//! Aizu Online Judge (AOJ) のサンプル取得・問題情報取得。
//!
//! AOJ は公式 REST API を提供しているため、スクレイピングではなく API を使用する。
//!
//! # API エンドポイント (Arena API v3)
//! - 問題情報:       `GET https://judgeapi.u-aizu.ac.jp/problems/{problem_id}`
//! - サンプル:       `GET https://judgeapi.u-aizu.ac.jp/samples/{problem_id}/{sample_index}`
//! - コース問題一覧: `GET https://judgeapi.u-aizu.ac.jp/courses/filter?id={course_id}`
//!
//! # URL パターン
//! - 問題: `https://onlinejudge.u-aizu.ac.jp/problems/{problem_id}`
//!         `https://judge.u-aizu.ac.jp/onlinejudge/description.jsp?id={problem_id}`
//! - コース: `https://onlinejudge.u-aizu.ac.jp/courses/lesson/{...}`
//!
//! Note: AOJ は AtCoder / Codeforces のような「コンテスト」の概念が薄く、
//!       常設問題集（Volume / Course）が主体。`fetch_contest` はコース対応の簡易実装。

use super::model::{ContestMeta, SampleCase, TaskMeta};
use crate::error::AppError;
use serde::Deserialize;

const JUDGE_API: &str = "https://judgeapi.u-aizu.ac.jp";

// ─── URL 判定 ──────────────────────────────────────────────────────

pub fn is_url(url: &str) -> bool {
    url.contains("u-aizu.ac.jp")
}

pub fn is_contest_url(url: &str) -> bool {
    is_url(url) && url.contains("/courses/")
}

pub fn is_problem_url(url: &str) -> bool {
    is_url(url) && (url.contains("/problems/") || url.contains("description.jsp"))
}

// ─── API レスポンス型 ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApiProblem {
    id: String,
    name: Option<String>,
    #[serde(rename = "numberOfSamples")]
    number_of_samples: Option<u32>,
}

#[derive(Debug, Deserialize)]
struct ApiSample {
    input: String,
    output: String,
}

#[derive(Debug, Deserialize)]
struct ApiCourse {
    id: String,
    name: String,
    problems: Option<Vec<ApiCourseProblem>>,
}

#[derive(Debug, Deserialize)]
struct ApiCourseProblem {
    id: String,
    name: Option<String>,
}

// ─── コンテスト取得（コース対応）───────────────────────────────────

/// AOJ のコース URL からタスク一覧を取得する。
/// 通常の「コンテスト」には非対応。
pub async fn fetch_contest(
    url: &str,
    client: &reqwest::Client,
) -> Result<ContestMeta, AppError> {
    let course_id = extract_course_id(url)?;
    let api_url = format!("{JUDGE_API}/courses/filter?id={course_id}");

    let courses: Vec<ApiCourse> = client
        .get(&api_url)
        .send()
        .await?
        .json()
        .await
        .map_err(|_| {
            AppError::SampleParse(format!("Failed to fetch AOJ course '{course_id}'"))
        })?;

    let course = courses
        .into_iter()
        .next()
        .ok_or_else(|| AppError::SampleParse(format!("AOJ course '{course_id}' not found")))?;

    let tasks = course
        .problems
        .unwrap_or_default()
        .into_iter()
        .map(|p| TaskMeta {
            id: p.id.clone(),
            name: p.name.unwrap_or_else(|| p.id.clone()),
            url: format!("https://onlinejudge.u-aizu.ac.jp/problems/{}", p.id),
        })
        .collect();

    Ok(ContestMeta {
        judge: "aoj".to_string(),
        contest_id: course.id.clone(),
        contest_name: course.name,
        url: url.to_string(),
        tasks,
    })
}

// ─── サンプル取得 ───────────────────────────────────────────────────

pub async fn fetch_samples(
    url: &str,
    client: &reqwest::Client,
) -> Result<Vec<SampleCase>, AppError> {
    let problem_id = extract_problem_id(url)?;

    // まず問題情報を取得してサンプル数を確認
    let prob_url = format!("{JUDGE_API}/problems/{problem_id}");
    let problem: ApiProblem = client
        .get(&prob_url)
        .send()
        .await?
        .json()
        .await
        .map_err(|_| AppError::SampleParse(format!("Problem '{problem_id}' not found")))?;

    let n = problem.number_of_samples.unwrap_or(0);
    if n == 0 {
        return Ok(Vec::new());
    }

    // サンプルを 1-indexed で取得
    let mut samples = Vec::new();
    for i in 1..=n {
        let sample_url = format!("{JUDGE_API}/samples/{problem_id}/{i}");
        if let Ok(s) = client
            .get(&sample_url)
            .send()
            .await?
            .json::<ApiSample>()
            .await
        {
            samples.push(SampleCase {
                input: s.input,
                output: s.output,
            });
        }
    }

    Ok(samples)
}

// ─── ヘルパー ──────────────────────────────────────────────────────

fn extract_problem_id(url: &str) -> Result<String, AppError> {
    // https://onlinejudge.u-aizu.ac.jp/problems/ITP1_1_A → "ITP1_1_A"
    if url.contains("/problems/") {
        return url
            .trim_end_matches('/')
            .split("/problems/")
            .nth(1)
            .and_then(|s| s.split('/').next())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::UnsupportedUrl(url.to_string()));
    }
    // https://judge.u-aizu.ac.jp/onlinejudge/description.jsp?id=ITP1_1_A → "ITP1_1_A"
    if url.contains("description.jsp") {
        return url
            .split("id=")
            .nth(1)
            .and_then(|s| s.split('&').next())
            .map(|s| s.to_string())
            .ok_or_else(|| AppError::UnsupportedUrl(url.to_string()));
    }
    Err(AppError::UnsupportedUrl(url.to_string()))
}

fn extract_course_id(url: &str) -> Result<String, AppError> {
    // https://onlinejudge.u-aizu.ac.jp/courses/lesson/2/ITP1/1 → "ITP1"
    // 簡易実装: /courses/ 以降の 3 番目のセグメントをコース ID とする
    let path = url
        .trim_end_matches('/')
        .split("/courses/")
        .nth(1)
        .ok_or_else(|| AppError::UnsupportedUrl(url.to_string()))?;

    let segments: Vec<&str> = path.split('/').collect();
    // lesson/{type}/{course_id} または topic/{type}/{course_id}
    segments
        .get(2)
        .or_else(|| segments.first())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::UnsupportedUrl(url.to_string()))
}

#[cfg(test)]
mod tests {
    use super::*;

    // ─── URL 判定 ─────────────────────────────────────────────────

    #[test]
    fn is_url_problem() {
        assert!(is_url(
            "https://onlinejudge.u-aizu.ac.jp/problems/ITP1_1_A"
        ));
    }

    #[test]
    fn is_url_course() {
        assert!(is_url(
            "https://onlinejudge.u-aizu.ac.jp/courses/lesson/2/ITP1/1"
        ));
    }

    #[test]
    fn is_url_description_jsp() {
        assert!(is_url(
            "https://judge.u-aizu.ac.jp/onlinejudge/description.jsp?id=ITP1_1_A"
        ));
    }

    #[test]
    fn is_url_false_for_other() {
        assert!(!is_url("https://atcoder.jp/contests/abc001"));
    }

    #[test]
    fn is_contest_url_course() {
        assert!(is_contest_url(
            "https://onlinejudge.u-aizu.ac.jp/courses/lesson/2/ITP1/1"
        ));
    }

    #[test]
    fn is_contest_url_false_for_problem() {
        assert!(!is_contest_url(
            "https://onlinejudge.u-aizu.ac.jp/problems/ITP1_1_A"
        ));
    }

    #[test]
    fn is_problem_url_problems_path() {
        assert!(is_problem_url(
            "https://onlinejudge.u-aizu.ac.jp/problems/ITP1_1_A"
        ));
    }

    #[test]
    fn is_problem_url_description_jsp() {
        assert!(is_problem_url(
            "https://judge.u-aizu.ac.jp/onlinejudge/description.jsp?id=ITP1_1_A"
        ));
    }

    // ─── extract_problem_id ──────────────────────────────────────

    #[test]
    fn extract_problem_id_from_problems_path() {
        let id = extract_problem_id(
            "https://onlinejudge.u-aizu.ac.jp/problems/ITP1_1_A",
        )
        .unwrap();
        assert_eq!(id, "ITP1_1_A");
    }

    #[test]
    fn extract_problem_id_from_description_jsp() {
        let id = extract_problem_id(
            "https://judge.u-aizu.ac.jp/onlinejudge/description.jsp?id=ITP1_1_A",
        )
        .unwrap();
        assert_eq!(id, "ITP1_1_A");
    }

    #[test]
    fn extract_problem_id_description_jsp_with_extra_param() {
        let id = extract_problem_id(
            "https://judge.u-aizu.ac.jp/onlinejudge/description.jsp?id=0001&lang=en",
        )
        .unwrap();
        assert_eq!(id, "0001");
    }

    #[test]
    fn extract_problem_id_unsupported() {
        let err = extract_problem_id("https://example.com/foo").unwrap_err();
        assert!(matches!(err, AppError::UnsupportedUrl(_)));
    }

    // ─── extract_course_id ──────────────────────────────────────

    #[test]
    fn extract_course_id_lesson_url() {
        let id = extract_course_id(
            "https://onlinejudge.u-aizu.ac.jp/courses/lesson/2/ITP1/1",
        )
        .unwrap();
        assert_eq!(id, "ITP1");
    }

    #[test]
    fn extract_course_id_short_path_falls_back_to_first() {
        // /courses/{type} のように短い場合は先頭セグメントを返す
        let id = extract_course_id(
            "https://onlinejudge.u-aizu.ac.jp/courses/lesson",
        )
        .unwrap();
        assert_eq!(id, "lesson");
    }

    #[test]
    fn extract_course_id_unsupported() {
        let err = extract_course_id("https://example.com/foo").unwrap_err();
        assert!(matches!(err, AppError::UnsupportedUrl(_)));
    }
}
