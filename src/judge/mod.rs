pub mod aoj;
pub mod atcoder;
pub mod codeforces;
pub mod model;
pub mod yukicoder;

use crate::error::AppError;
pub use model::SimpleContest;
use model::{ContestMeta, SampleCase};

// ─── ジャッジ判別 ──────────────────────────────────────────────────

/// URL からどのジャッジに属するかを判定する。
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum JudgeKind {
    AtCoder,
    Codeforces,
    Yukicoder,
    Aoj,
}

impl JudgeKind {
    /// URL を見てジャッジを特定する。どれにも一致しない場合は `UnsupportedUrl` を返す。
    pub fn from_url(url: &str) -> Result<Self, AppError> {
        if atcoder::is_url(url) {
            Ok(Self::AtCoder)
        } else if codeforces::is_url(url) {
            Ok(Self::Codeforces)
        } else if yukicoder::is_url(url) {
            Ok(Self::Yukicoder)
        } else if aoj::is_url(url) {
            Ok(Self::Aoj)
        } else {
            Err(AppError::UnsupportedUrl(url.to_string()))
        }
    }

    pub fn as_str(&self) -> &'static str {
        match self {
            Self::AtCoder => "atcoder",
            Self::Codeforces => "codeforces",
            Self::Yukicoder => "yukicoder",
            Self::Aoj => "aoj",
        }
    }
}

// ─── 公開 API ────────────────────────────────────────────────────

/// コンテスト URL からメタデータ（タスク一覧含む）を取得する。
pub async fn fetch_contest(url: &str, client: &reqwest::Client) -> Result<ContestMeta, AppError> {
    match JudgeKind::from_url(url)? {
        JudgeKind::AtCoder => atcoder::fetch_contest(url, client).await,
        JudgeKind::Codeforces => codeforces::fetch_contest(url, client).await,
        JudgeKind::Yukicoder => yukicoder::fetch_contest(url, client).await,
        JudgeKind::Aoj => aoj::fetch_contest(url, client).await,
    }
}

/// 問題 URL からサンプルケース一覧を取得する。
pub async fn fetch_samples(
    url: &str,
    client: &reqwest::Client,
) -> Result<Vec<SampleCase>, AppError> {
    match JudgeKind::from_url(url)? {
        JudgeKind::AtCoder => atcoder::fetch_samples(url, client).await,
        JudgeKind::Codeforces => codeforces::fetch_samples(url, client).await,
        JudgeKind::Yukicoder => yukicoder::fetch_samples(url, client).await,
        JudgeKind::Aoj => aoj::fetch_samples(url, client).await,
    }
}

/// 与えられた URL がコンテスト URL か問題 URL かを判定する。
pub fn is_contest_url(url: &str) -> bool {
    match JudgeKind::from_url(url) {
        Ok(JudgeKind::AtCoder) => atcoder::is_contest_url(url),
        Ok(JudgeKind::Codeforces) => codeforces::is_contest_url(url),
        Ok(JudgeKind::Yukicoder) => yukicoder::is_contest_url(url),
        Ok(JudgeKind::Aoj) => aoj::is_contest_url(url),
        Err(_) => false,
    }
}

pub async fn fetch_contest_list(
    judge: &JudgeKind,
    client: &reqwest::Client,
) -> Result<Vec<SimpleContest>, AppError> {
    match judge {
        JudgeKind::AtCoder => atcoder::fetch_contest_list(client).await,
        JudgeKind::Codeforces => codeforces::fetch_contest_list(client).await,
        JudgeKind::Yukicoder => yukicoder::fetch_contest_list(client).await,
        JudgeKind::Aoj => aoj::fetch_contest_list(client).await,
    }
}

/// クエリ文字列を既知のプレフィックスパターンで直接コンテスト URL に解決する。
/// マッチしなければ `None` を返す。
fn try_direct_resolve(query: &str) -> Option<String> {
    // URL はそのまま返す
    if query.starts_with("http://") || query.starts_with("https://") {
        return Some(query.to_string());
    }

    let q = query.to_lowercase();

    // AtCoder ID パターン (例: abc300, arc120)
    let atcoder_prefixes = ["abc", "arc", "agc", "ahc", "apc", "jsc", "past"];
    for p in atcoder_prefixes {
        if q.starts_with(p) && q[p.len()..].chars().all(|c| c.is_ascii_digit()) && q.len() > p.len()
        {
            return Some(format!("https://atcoder.jp/contests/{}", q));
        }
    }

    // Codeforces ID パターン (例: cf1800)
    if q.starts_with("cf") && q[2..].chars().all(|c| c.is_ascii_digit()) && q.len() > 2 {
        return Some(format!("https://codeforces.com/contest/{}", &q[2..]));
    }

    // yukicoder ID パターン (例: yuki400)
    if q.starts_with("yuki") && q[4..].chars().all(|c| c.is_ascii_digit()) && q.len() > 4 {
        return Some(format!("https://yukicoder.me/contests/{}", &q[4..]));
    }

    // AOJ コース ID パターン (例: itp1, alds1)
    let aoj_courses = ["itp1", "alds1", "dsl", "grl", "cgl", "alpc"];
    if aoj_courses.contains(&q.as_str()) {
        return Some(format!(
            "https://onlinejudge.u-aizu.ac.jp/courses/lesson/2/{}/1",
            query.to_uppercase(),
        ));
    }

    None
}

/// クエリのすべてのトークン（空白区切り・小文字化済み）が対象文字列に含まれるかを判定する。
fn matches_all_tokens(tokens: &[String], haystack: &str) -> bool {
    let h = haystack.to_lowercase();
    tokens.iter().all(|t| h.contains(t.as_str()))
}

/// 曖昧検索の候補表示上限。
const FUZZY_DISPLAY_LIMIT: usize = 20;

/// ジャッジ名付きのコンテスト一覧からクエリにマッチするコンテストを検索し、
/// 0件 → エラー、1件 → URL 返却、複数件 → 候補表示+エラー を返す。
///
/// ネットワーク非依存の純粋ロジック。`resolve_query` から分離されている。
fn fuzzy_search_contests<'a>(
    query: &str,
    judges: &[(&'a str, Vec<SimpleContest>)],
) -> Result<String, AppError> {
    let tokens: Vec<String> = query
        .to_lowercase()
        .split_whitespace()
        .map(String::from)
        .collect();

    let mut matches: Vec<(&str, SimpleContest)> = Vec::new();

    for (judge_name, list) in judges {
        for c in list {
            if matches_all_tokens(&tokens, &c.id) || matches_all_tokens(&tokens, &c.name) {
                matches.push((judge_name, c.clone()));
            }
        }
    }

    match matches.len() {
        0 => Err(AppError::SampleParse(format!(
            "No contests found matching '{}'",
            query
        ))),
        1 => {
            let (judge, contest) = &matches[0];
            println!(
                "Found matching contest on {}: {} - {}",
                judge, contest.id, contest.name
            );
            Ok(contest.url.clone())
        }
        n => {
            eprintln!("Multiple contests found for '{}' ({} matches):", query, n);
            for (judge, contest) in matches.iter().take(FUZZY_DISPLAY_LIMIT) {
                eprintln!(
                    "  [{}] {} — {} ({})",
                    judge, contest.id, contest.name, contest.url
                );
            }
            if n > FUZZY_DISPLAY_LIMIT {
                eprintln!("  ... and {} more", n - FUZZY_DISPLAY_LIMIT);
            }
            Err(AppError::SampleParse(
                "Multiple matches found. Please specify a more specific query or URL.".to_string(),
            ))
        }
    }
}

/// ユーザー入力（URLまたはクエリ）を解決し、最終的なコンテストURLを返す。
pub async fn resolve_query(query: &str, client: &reqwest::Client) -> Result<String, AppError> {
    // 1. 直接解決を試みる
    if let Some(url) = try_direct_resolve(query) {
        return Ok(url);
    }

    // 2. 曖昧検索（すべてのジャッジから並列取得してトークン AND 部分一致で検索）
    println!(
        "Query '{}' did not match direct patterns. Searching all judges...",
        query
    );

    let (at_res, cf_res, yuki_res, aoj_res) = tokio::join!(
        atcoder::fetch_contest_list(client),
        codeforces::fetch_contest_list(client),
        yukicoder::fetch_contest_list(client),
        aoj::fetch_contest_list(client),
    );

    let mut judge_lists: Vec<(&str, Vec<SimpleContest>)> = Vec::new();
    for (name, res) in [
        ("atcoder", at_res),
        ("codeforces", cf_res),
        ("yukicoder", yuki_res),
        ("aoj", aoj_res),
    ] {
        if let Ok(list) = res {
            judge_lists.push((name, list));
        }
    }

    fuzzy_search_contests(query, &judge_lists)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::error::AppError;

    // ─── JudgeKind::from_url ───────────────────────────────────────

    #[test]
    fn from_url_atcoder_contest() {
        let kind = JudgeKind::from_url("https://atcoder.jp/contests/abc001").unwrap();
        assert_eq!(kind, JudgeKind::AtCoder);
    }

    #[test]
    fn from_url_atcoder_problem() {
        let kind =
            JudgeKind::from_url("https://atcoder.jp/contests/abc001/tasks/abc001_a").unwrap();
        assert_eq!(kind, JudgeKind::AtCoder);
    }

    #[test]
    fn from_url_atcoder_legacy_contest() {
        let kind = JudgeKind::from_url("https://abc001.contest.atcoder.jp/").unwrap();
        assert_eq!(kind, JudgeKind::AtCoder);
    }

    #[test]
    fn from_url_atcoder_legacy_problem() {
        let kind = JudgeKind::from_url("https://abc001.contest.atcoder.jp/tasks/abc001_a").unwrap();
        assert_eq!(kind, JudgeKind::AtCoder);
    }

    #[test]
    fn from_url_codeforces_contest() {
        let kind = JudgeKind::from_url("https://codeforces.com/contest/1234").unwrap();
        assert_eq!(kind, JudgeKind::Codeforces);
    }

    #[test]
    fn from_url_codeforces_gym() {
        let kind = JudgeKind::from_url("https://codeforces.com/gym/102028").unwrap();
        assert_eq!(kind, JudgeKind::Codeforces);
    }

    #[test]
    fn from_url_yukicoder_contest() {
        let kind = JudgeKind::from_url("https://yukicoder.me/contests/400").unwrap();
        assert_eq!(kind, JudgeKind::Yukicoder);
    }

    #[test]
    fn from_url_yukicoder_problem() {
        let kind = JudgeKind::from_url("https://yukicoder.me/problems/no/1").unwrap();
        assert_eq!(kind, JudgeKind::Yukicoder);
    }

    #[test]
    fn from_url_aoj_problem() {
        let kind =
            JudgeKind::from_url("https://onlinejudge.u-aizu.ac.jp/problems/ITP1_1_A").unwrap();
        assert_eq!(kind, JudgeKind::Aoj);
    }

    #[test]
    fn from_url_unknown_returns_error() {
        let err = JudgeKind::from_url("https://example.com/foo").unwrap_err();
        assert!(matches!(err, AppError::UnsupportedUrl(_)));
    }

    #[test]
    fn as_str_values() {
        assert_eq!(JudgeKind::AtCoder.as_str(), "atcoder");
        assert_eq!(JudgeKind::Codeforces.as_str(), "codeforces");
        assert_eq!(JudgeKind::Yukicoder.as_str(), "yukicoder");
        assert_eq!(JudgeKind::Aoj.as_str(), "aoj");
    }

    // ─── is_contest_url (public) ──────────────────────────────────

    #[test]
    fn is_contest_url_atcoder_true() {
        assert!(is_contest_url("https://atcoder.jp/contests/abc001"));
    }

    #[test]
    fn is_contest_url_atcoder_problem_false() {
        assert!(!is_contest_url(
            "https://atcoder.jp/contests/abc001/tasks/abc001_a"
        ));
    }

    #[test]
    fn is_contest_url_unknown_false() {
        assert!(!is_contest_url("https://example.com"));
    }

    // ─── try_direct_resolve ───────────────────────────────────────

    #[test]
    fn direct_resolve_url_passthrough() {
        assert_eq!(
            try_direct_resolve("https://atcoder.jp/contests/abc300"),
            Some("https://atcoder.jp/contests/abc300".to_string()),
        );
    }

    #[test]
    fn direct_resolve_http_url_passthrough() {
        assert_eq!(
            try_direct_resolve("http://example.com"),
            Some("http://example.com".to_string()),
        );
    }

    #[test]
    fn direct_resolve_abc300() {
        assert_eq!(
            try_direct_resolve("abc300"),
            Some("https://atcoder.jp/contests/abc300".to_string()),
        );
    }

    #[test]
    fn direct_resolve_abc_uppercase() {
        // 大文字入力も小文字に正規化されて解決される
        assert_eq!(
            try_direct_resolve("ABC300"),
            Some("https://atcoder.jp/contests/abc300".to_string()),
        );
    }

    #[test]
    fn direct_resolve_arc120() {
        assert_eq!(
            try_direct_resolve("arc120"),
            Some("https://atcoder.jp/contests/arc120".to_string()),
        );
    }

    #[test]
    fn direct_resolve_agc055() {
        assert_eq!(
            try_direct_resolve("agc055"),
            Some("https://atcoder.jp/contests/agc055".to_string()),
        );
    }

    #[test]
    fn direct_resolve_ahc001() {
        assert_eq!(
            try_direct_resolve("ahc001"),
            Some("https://atcoder.jp/contests/ahc001".to_string()),
        );
    }

    #[test]
    fn direct_resolve_cf1800() {
        assert_eq!(
            try_direct_resolve("cf1800"),
            Some("https://codeforces.com/contest/1800".to_string()),
        );
    }

    #[test]
    fn direct_resolve_yuki400() {
        assert_eq!(
            try_direct_resolve("yuki400"),
            Some("https://yukicoder.me/contests/400".to_string()),
        );
    }

    #[test]
    fn direct_resolve_itp1() {
        assert_eq!(
            try_direct_resolve("itp1"),
            Some("https://onlinejudge.u-aizu.ac.jp/courses/lesson/2/ITP1/1".to_string()),
        );
    }

    #[test]
    fn direct_resolve_alds1() {
        assert_eq!(
            try_direct_resolve("alds1"),
            Some("https://onlinejudge.u-aizu.ac.jp/courses/lesson/2/ALDS1/1".to_string()),
        );
    }

    #[test]
    fn direct_resolve_prefix_only_no_digits() {
        // "abc" だけでは数字がないので直接解決しない
        assert_eq!(try_direct_resolve("abc"), None);
    }

    #[test]
    fn direct_resolve_cf_only_no_digits() {
        assert_eq!(try_direct_resolve("cf"), None);
    }

    #[test]
    fn direct_resolve_digits_only_returns_none() {
        // 数字のみは直接解決しない（CF/yukicoder の ID 衝突回避）
        assert_eq!(try_direct_resolve("1800"), None);
    }

    #[test]
    fn direct_resolve_unknown_keyword_returns_none() {
        assert_eq!(try_direct_resolve("beginner"), None);
    }

    #[test]
    fn direct_resolve_multi_word_returns_none() {
        assert_eq!(try_direct_resolve("beginner 300"), None);
    }

    // ─── matches_all_tokens ──────────────────────────────────────

    #[test]
    fn matches_all_tokens_single_token() {
        let tokens = vec!["beginner".to_string()];
        assert!(matches_all_tokens(&tokens, "AtCoder Beginner Contest 300"));
    }

    #[test]
    fn matches_all_tokens_multiple_tokens() {
        let tokens = vec!["beginner".to_string(), "300".to_string()];
        assert!(matches_all_tokens(&tokens, "AtCoder Beginner Contest 300"));
    }

    #[test]
    fn matches_all_tokens_partial_miss() {
        let tokens = vec!["beginner".to_string(), "999".to_string()];
        assert!(!matches_all_tokens(&tokens, "AtCoder Beginner Contest 300"));
    }

    #[test]
    fn matches_all_tokens_empty_tokens() {
        let tokens: Vec<String> = vec![];
        // 空トークンはすべてマッチ（all() の空集合は true）
        assert!(matches_all_tokens(&tokens, "anything"));
    }

    #[test]
    fn matches_all_tokens_case_insensitive() {
        let tokens = vec!["abc".to_string()];
        assert!(matches_all_tokens(&tokens, "ABC300"));
    }

    // ─── try_direct_resolve: 残りの AtCoder プレフィックス ─────────

    #[test]
    fn direct_resolve_apc001() {
        assert_eq!(
            try_direct_resolve("apc001"),
            Some("https://atcoder.jp/contests/apc001".to_string()),
        );
    }

    #[test]
    fn direct_resolve_jsc2019() {
        assert_eq!(
            try_direct_resolve("jsc2019"),
            Some("https://atcoder.jp/contests/jsc2019".to_string()),
        );
    }

    #[test]
    fn direct_resolve_past15() {
        assert_eq!(
            try_direct_resolve("past15"),
            Some("https://atcoder.jp/contests/past15".to_string()),
        );
    }

    // ─── try_direct_resolve: 残りの AOJ コース ────────────────────

    #[test]
    fn direct_resolve_dsl() {
        assert_eq!(
            try_direct_resolve("dsl"),
            Some("https://onlinejudge.u-aizu.ac.jp/courses/lesson/2/DSL/1".to_string()),
        );
    }

    #[test]
    fn direct_resolve_grl() {
        assert_eq!(
            try_direct_resolve("grl"),
            Some("https://onlinejudge.u-aizu.ac.jp/courses/lesson/2/GRL/1".to_string()),
        );
    }

    #[test]
    fn direct_resolve_cgl() {
        assert_eq!(
            try_direct_resolve("cgl"),
            Some("https://onlinejudge.u-aizu.ac.jp/courses/lesson/2/CGL/1".to_string()),
        );
    }

    #[test]
    fn direct_resolve_alpc() {
        assert_eq!(
            try_direct_resolve("alpc"),
            Some("https://onlinejudge.u-aizu.ac.jp/courses/lesson/2/ALPC/1".to_string()),
        );
    }

    // ─── fuzzy_search_contests ───────────────────────────────────

    /// テスト用のコンテスト一覧を生成するヘルパー。
    fn sample_contest_lists() -> Vec<(&'static str, Vec<SimpleContest>)> {
        vec![
            (
                "atcoder",
                vec![
                    SimpleContest {
                        id: "abc300".to_string(),
                        name: "AtCoder Beginner Contest 300".to_string(),
                        url: "https://atcoder.jp/contests/abc300".to_string(),
                    },
                    SimpleContest {
                        id: "abc301".to_string(),
                        name: "AtCoder Beginner Contest 301".to_string(),
                        url: "https://atcoder.jp/contests/abc301".to_string(),
                    },
                    SimpleContest {
                        id: "arc150".to_string(),
                        name: "AtCoder Regular Contest 150".to_string(),
                        url: "https://atcoder.jp/contests/arc150".to_string(),
                    },
                ],
            ),
            (
                "codeforces",
                vec![SimpleContest {
                    id: "1800".to_string(),
                    name: "Codeforces Round 1800".to_string(),
                    url: "https://codeforces.com/contest/1800".to_string(),
                }],
            ),
            (
                "yukicoder",
                vec![SimpleContest {
                    id: "400".to_string(),
                    name: "yukicoder contest 400".to_string(),
                    url: "https://yukicoder.me/contests/400".to_string(),
                }],
            ),
        ]
    }

    #[test]
    fn fuzzy_search_zero_matches() {
        let lists = sample_contest_lists();
        let result = fuzzy_search_contests("nonexistent999", &lists);
        assert!(result.is_err());
        let err = result.unwrap_err();
        assert!(matches!(err, AppError::SampleParse(_)));
        assert!(err.to_string().contains("No contests found"));
    }

    #[test]
    fn fuzzy_search_single_match_by_name() {
        let lists = sample_contest_lists();
        // "regular 150" は "AtCoder Regular Contest 150" のみにマッチ
        let result = fuzzy_search_contests("regular 150", &lists);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://atcoder.jp/contests/arc150");
    }

    #[test]
    fn fuzzy_search_single_match_by_id() {
        let lists = sample_contest_lists();
        // "arc150" は id "arc150" にマッチ（1件のみ）
        let result = fuzzy_search_contests("arc150", &lists);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://atcoder.jp/contests/arc150");
    }

    #[test]
    fn fuzzy_search_multiple_matches() {
        let lists = sample_contest_lists();
        // "beginner" は abc300 と abc301 の両方にマッチ
        let result = fuzzy_search_contests("beginner", &lists);
        assert!(result.is_err());
        assert!(result.unwrap_err().to_string().contains("Multiple matches"));
    }

    #[test]
    fn fuzzy_search_token_and_narrows_results() {
        let lists = sample_contest_lists();
        // "beginner 300" は abc300 のみにマッチ（abc301 は除外される）
        let result = fuzzy_search_contests("beginner 300", &lists);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://atcoder.jp/contests/abc300");
    }

    #[test]
    fn fuzzy_search_case_insensitive() {
        let lists = sample_contest_lists();
        // 大文字小文字を区別しない
        let result = fuzzy_search_contests("REGULAR 150", &lists);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://atcoder.jp/contests/arc150");
    }

    #[test]
    fn fuzzy_search_cross_judge_match() {
        let lists = sample_contest_lists();
        // "contest 400" は yukicoder の "yukicoder contest 400" にマッチ
        // codeforces には "contest" を含む名前はあるが "400" を含まない
        let result = fuzzy_search_contests("contest 400", &lists);
        assert!(result.is_ok());
        assert_eq!(result.unwrap(), "https://yukicoder.me/contests/400");
    }

    #[test]
    fn fuzzy_search_empty_lists() {
        let lists: Vec<(&str, Vec<SimpleContest>)> = vec![];
        let result = fuzzy_search_contests("anything", &lists);
        assert!(result.is_err());
        assert!(
            result
                .unwrap_err()
                .to_string()
                .contains("No contests found")
        );
    }
}
