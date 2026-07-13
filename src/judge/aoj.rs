//! Aizu Online Judge (AOJ) のサンプル取得・問題情報取得。
//!
//! AOJ は公式 REST API を提供しているため、スクレイピングではなく API を使用する。
//!
//! # API エンドポイント
//! - サンプル一括取得: `GET https://judgedat.u-aizu.ac.jp/testcases/samples/{problem_id}`
//!   レスポンス: `[{"problemId": "...", "serial": N, "in": "...", "out": "..."}, ...]`
//! - コース一覧:       `GET https://judgeapi.u-aizu.ac.jp/courses`
//!   レスポンス: `{"courses": [{"id": N, "shortName": "ITP1", "name": "...", ...}, ...]}`
//! - トピック一覧:     `GET https://judgeapi.u-aizu.ac.jp/courses/{course_id}/topics`
//!   レスポンス: `{"_embedded": {"topics": [{"_links": {"self": {"href": ".../topics/{id}"}}, ...}, ...]}}`
//! - トピック問題一覧: `GET https://judgeapi.u-aizu.ac.jp/topics/{topic_id}/problems`
//!   レスポンス: `{"_embedded": {"problems": [{"name": "...", "_links": {"self": {"href": ".../problems/{problem_id}"}}, ...}, ...]}}`
//! - Volume 問題一覧:  `GET https://judgeapi.u-aizu.ac.jp/problems/volumes/{vol_no}?page=0&size={n}`
//!   レスポンス: `{"numberOfProblems": N, "problems": [{"id": "0100", "name": "...", ...}, ...]}`
//!
//! # URL パターン
//! - 問題: `https://onlinejudge.u-aizu.ac.jp/problems/{problem_id}`
//!   または `https://judge.u-aizu.ac.jp/onlinejudge/description.jsp?id={problem_id}`
//! - コース:  `https://onlinejudge.u-aizu.ac.jp/courses/lesson/{...}`
//! - Volume: `https://onlinejudge.u-aizu.ac.jp/challenges/volumes/{vol_no}`
//!
//! Note: AOJ は AtCoder / Codeforces のような「コンテスト」の概念が薄く、
//!       常設問題集（Volume / Course）が主体。`fetch_contest` はコース・Volume 対応の簡易実装。

use super::model::{ContestMeta, SampleCase, TaskMeta};
use crate::error::AppError;
use serde::Deserialize;

const JUDGE_API: &str = "https://judgeapi.u-aizu.ac.jp";
const JUDGE_DAT_API: &str = "https://judgedat.u-aizu.ac.jp";

// ─── URL 判定 ──────────────────────────────────────────────────────

pub fn is_url(url: &str) -> bool {
    url.contains("u-aizu.ac.jp")
}

pub fn is_contest_url(url: &str) -> bool {
    is_url(url) && (url.contains("/courses/") || url.contains("/volumes/"))
}

pub fn is_volume_url(url: &str) -> bool {
    is_url(url) && url.contains("/volumes/")
}

pub fn is_problem_url(url: &str) -> bool {
    is_url(url) && (url.contains("/problems/") || url.contains("description.jsp"))
}

// ─── API レスポンス型 ──────────────────────────────────────────────

#[derive(Debug, Deserialize)]
struct ApiSample {
    #[serde(rename = "in")]
    input: String,
    #[serde(rename = "out")]
    output: String,
}

/// `GET /courses` のレスポンス
#[derive(Debug, Deserialize)]
struct ApiCoursesResponse {
    courses: Vec<ApiCourse>,
}

/// コース一覧の各エントリ
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiCourse {
    id: u64,
    short_name: String,
    name: String,
}

/// `GET /courses/{id}/topics` の `_embedded.topics[]` 要素
#[derive(Debug, Deserialize)]
struct ApiTopic {
    #[serde(rename = "_links")]
    links: ApiLinks,
}

/// `GET /topics/{id}/problems` の `_embedded.problems[]` 要素
#[derive(Debug, Deserialize)]
struct ApiTopicProblem {
    name: String,
    #[serde(rename = "_links")]
    links: ApiLinks,
}

/// HAL スタイルの `_links` オブジェクト（`self.href` のみ利用）
#[derive(Debug, Deserialize)]
struct ApiLinks {
    #[serde(rename = "self")]
    self_link: ApiHref,
}

#[derive(Debug, Deserialize)]
struct ApiHref {
    href: String,
}

/// `GET /courses/{id}/topics` のレスポンス（HAL 形式）
#[derive(Debug, Deserialize)]
struct ApiTopicsResponse {
    #[serde(rename = "_embedded")]
    embedded: ApiTopicsEmbedded,
}

#[derive(Debug, Deserialize)]
struct ApiTopicsEmbedded {
    topics: Vec<ApiTopic>,
}

/// `GET /topics/{id}/problems` のレスポンス（HAL 形式）
#[derive(Debug, Deserialize)]
struct ApiTopicProblemsResponse {
    #[serde(rename = "_embedded")]
    embedded: ApiTopicProblemsEmbedded,
}

#[derive(Debug, Deserialize)]
struct ApiTopicProblemsEmbedded {
    problems: Vec<ApiTopicProblem>,
}

/// `GET /problems/volumes/{vol_no}` のレスポンス
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase")]
struct ApiVolumeResponse {
    number_of_problems: u64,
    problems: Vec<ApiVolumeProblem>,
}

/// Volume 問題一覧の各エントリ
#[derive(Debug, Deserialize)]
struct ApiVolumeProblem {
    id: String,
    name: String,
}

// ─── コンテスト取得（コース対応）───────────────────────────────────

/// AOJ のコース / Volume URL からタスク一覧を取得する。
/// 通常の「コンテスト」には非対応。
///
/// Volume URL の場合は `fetch_volume` に委譲する。
///
/// # コース URL の手順
/// 1. `GET /courses` でコース一覧を取得し、URL から抽出した `shortName` に一致するコースを探す。
/// 2. `GET /courses/{courseId}/topics` でトピック一覧を取得し、
///    各トピックの `_links.self.href` 末尾からトピック数値 ID を抽出する。
/// 3. 各トピックに対して `GET /topics/{topicId}/problems` で問題一覧を取得し、
///    `_links.self.href` 末尾から問題 ID 文字列・`name` を得る。
pub async fn fetch_contest(
    url: &str,
    client: &reqwest::Client,
) -> Result<ContestMeta, AppError> {
    if is_volume_url(url) {
        return fetch_volume(url, client).await;
    }

    let short_name = extract_course_id(url)?;

    // ── Step 1: コース一覧から shortName に一致するコースを探す ──
    let courses_url = format!("{JUDGE_API}/courses");
    let courses_resp: ApiCoursesResponse = client
        .get(&courses_url)
        .send()
        .await?
        .json()
        .await
        .map_err(|_| AppError::SampleParse("Failed to fetch AOJ courses list".to_string()))?;

    let course = courses_resp
        .courses
        .into_iter()
        .find(|c| c.short_name.eq_ignore_ascii_case(&short_name))
        .ok_or_else(|| {
            AppError::SampleParse(format!("AOJ course '{short_name}' not found"))
        })?;

    // ── Step 2: コースのトピック一覧を取得 ──
    let topics_url = format!("{JUDGE_API}/courses/{}/topics", course.id);
    let topics_resp: ApiTopicsResponse = client
        .get(&topics_url)
        .send()
        .await?
        .json()
        .await
        .map_err(|_| {
            AppError::SampleParse(format!(
                "Failed to fetch topics for AOJ course '{}'",
                course.short_name
            ))
        })?;

    // ── Step 3: 各トピックの問題を収集 ──
    let mut tasks: Vec<TaskMeta> = Vec::new();
    for topic in topics_resp.embedded.topics {
        // `_links.self.href` 末尾のセグメントがトピック数値 ID
        let topic_id = topic
            .links
            .self_link
            .href
            .trim_end_matches('/')
            .rsplit('/')
            .next()
            .unwrap_or("")
            .to_string();

        let problems_url = format!("{JUDGE_API}/topics/{topic_id}/problems");
        let problems_resp: ApiTopicProblemsResponse = match client
            .get(&problems_url)
            .send()
            .await?
            .json()
            .await
        {
            Ok(r) => r,
            Err(_) => continue, // トピックの問題取得に失敗しても続行
        };

        for problem in problems_resp.embedded.problems {
            // `_links.self.href` 末尾のセグメントが問題 ID 文字列（例: "ITP1_1_A"）
            let problem_id = problem
                .links
                .self_link
                .href
                .trim_end_matches('/')
                .rsplit('/')
                .next()
                .unwrap_or("")
                .to_string();

            tasks.push(TaskMeta {
                id: problem_id.clone(),
                name: problem.name,
                url: format!("https://onlinejudge.u-aizu.ac.jp/problems/{problem_id}"),
            });
        }
    }

    Ok(ContestMeta {
        judge: "aoj".to_string(),
        contest_id: course.short_name.clone(),
        contest_name: course.name,
        url: url.to_string(),
        tasks,
    })
}

// ─── Volume 取得 ────────────────────────────────────────────────────

/// AOJ の Volume URL からタスク一覧を取得する。
///
/// # 手順
/// 1. `GET /problems/volumes/{vol_no}?page=0&size=1` で `numberOfProblems` を取得する。
/// 2. `GET /problems/volumes/{vol_no}?page=0&size={numberOfProblems}` で全問題を一括取得する。
async fn fetch_volume(
    url: &str,
    client: &reqwest::Client,
) -> Result<ContestMeta, AppError> {
    let vol_no = extract_volume_id(url)?;

    // ── Step 1: 問題数を先読み ──
    let probe_url = format!("{JUDGE_API}/problems/volumes/{vol_no}?page=0&size=1");
    let probe: ApiVolumeResponse = client
        .get(&probe_url)
        .send()
        .await?
        .json()
        .await
        .map_err(|_| {
            AppError::SampleParse(format!(
                "Failed to fetch AOJ volume {vol_no} metadata"
            ))
        })?;

    let count = probe.number_of_problems.max(1);

    // ── Step 2: 全問題を一括取得 ──
    let all_url = format!("{JUDGE_API}/problems/volumes/{vol_no}?page=0&size={count}");
    let all: ApiVolumeResponse = client
        .get(&all_url)
        .send()
        .await?
        .json()
        .await
        .map_err(|_| {
            AppError::SampleParse(format!(
                "Failed to fetch problem list for AOJ volume {vol_no}"
            ))
        })?;

    let tasks = all
        .problems
        .into_iter()
        .map(|p| TaskMeta {
            url: format!("https://onlinejudge.u-aizu.ac.jp/problems/{}", p.id),
            id: p.id.clone(),
            name: p.name,
        })
        .collect();

    Ok(ContestMeta {
        judge: "aoj".to_string(),
        contest_id: format!("volume{vol_no}"),
        contest_name: format!("AOJ Volume {vol_no}"),
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

    // judgedat API で全サンプルを一括取得
    // GET https://judgedat.u-aizu.ac.jp/testcases/samples/{problem_id}
    // → [{"problemId": "...", "serial": N, "in": "...", "out": "..."}, ...]
    let sample_url = format!("{JUDGE_DAT_API}/testcases/samples/{problem_id}");
    let raw: Vec<ApiSample> = client
        .get(&sample_url)
        .send()
        .await?
        .json()
        .await
        .map_err(|_| {
            AppError::SampleParse(format!("Failed to fetch samples for '{problem_id}'"))
        })?;

    let samples = raw
        .into_iter()
        .map(|s| SampleCase {
            input: s.input,
            output: s.output,
        })
        .collect();

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

fn extract_volume_id(url: &str) -> Result<String, AppError> {
    // https://onlinejudge.u-aizu.ac.jp/challenges/volumes/1 → "1"
    url.trim_end_matches('/')
        .split("/volumes/")
        .nth(1)
        .and_then(|s| s.split('/').next())
        .filter(|s| !s.is_empty())
        .map(|s| s.to_string())
        .ok_or_else(|| AppError::UnsupportedUrl(url.to_string()))
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

/// AOJ API からコース一覧を取得し、SimpleContest 形式にする。
pub async fn fetch_contest_list(
    client: &reqwest::Client,
) -> Result<Vec<super::model::SimpleContest>, AppError> {
    let url = format!("{}/courses", JUDGE_API);
    let resp: ApiCoursesResponse = client
        .get(&url)
        .header(reqwest::header::USER_AGENT, "je-cli")
        .send()
        .await?
        .json()
        .await
        .map_err(|e| AppError::SampleParse(format!("Failed to fetch AOJ courses: {}", e)))?;

    let contests = resp.courses
        .into_iter()
        .map(|c| super::model::SimpleContest {
            id: c.short_name.clone(),
            name: c.name,
            url: format!("https://onlinejudge.u-aizu.ac.jp/courses/lesson/2/{}/1", c.short_name),
        })
        .collect();

    Ok(contests)
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
    fn is_contest_url_volume() {
        assert!(is_contest_url(
            "https://onlinejudge.u-aizu.ac.jp/challenges/volumes/1"
        ));
    }

    #[test]
    fn is_volume_url_true() {
        assert!(is_volume_url(
            "https://onlinejudge.u-aizu.ac.jp/challenges/volumes/1"
        ));
    }

    #[test]
    fn is_volume_url_false_for_course() {
        assert!(!is_volume_url(
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

    // ─── extract_volume_id ──────────────────────────────────────

    #[test]
    fn extract_volume_id_challenges_url() {
        let id = extract_volume_id(
            "https://onlinejudge.u-aizu.ac.jp/challenges/volumes/1",
        )
        .unwrap();
        assert_eq!(id, "1");
    }

    #[test]
    fn extract_volume_id_trailing_slash() {
        let id = extract_volume_id(
            "https://onlinejudge.u-aizu.ac.jp/challenges/volumes/20/",
        )
        .unwrap();
        assert_eq!(id, "20");
    }

    #[test]
    fn extract_volume_id_unsupported() {
        let err = extract_volume_id("https://example.com/foo").unwrap_err();
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
