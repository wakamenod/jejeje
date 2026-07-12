pub mod atcoder;
pub mod aoj;
pub mod codeforces;
pub mod model;
pub mod yukicoder;

use crate::error::AppError;
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
pub async fn fetch_contest(
    url: &str,
    client: &reqwest::Client,
) -> Result<ContestMeta, AppError> {
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
        let kind = JudgeKind::from_url(
            "https://atcoder.jp/contests/abc001/tasks/abc001_a",
        )
        .unwrap();
        assert_eq!(kind, JudgeKind::AtCoder);
    }

    #[test]
    fn from_url_atcoder_legacy_contest() {
        let kind =
            JudgeKind::from_url("https://abc001.contest.atcoder.jp/").unwrap();
        assert_eq!(kind, JudgeKind::AtCoder);
    }

    #[test]
    fn from_url_atcoder_legacy_problem() {
        let kind = JudgeKind::from_url(
            "https://abc001.contest.atcoder.jp/tasks/abc001_a",
        )
        .unwrap();
        assert_eq!(kind, JudgeKind::AtCoder);
    }

    #[test]
    fn from_url_codeforces_contest() {
        let kind =
            JudgeKind::from_url("https://codeforces.com/contest/1234").unwrap();
        assert_eq!(kind, JudgeKind::Codeforces);
    }

    #[test]
    fn from_url_codeforces_gym() {
        let kind = JudgeKind::from_url("https://codeforces.com/gym/102028").unwrap();
        assert_eq!(kind, JudgeKind::Codeforces);
    }

    #[test]
    fn from_url_yukicoder_contest() {
        let kind =
            JudgeKind::from_url("https://yukicoder.me/contests/400").unwrap();
        assert_eq!(kind, JudgeKind::Yukicoder);
    }

    #[test]
    fn from_url_yukicoder_problem() {
        let kind =
            JudgeKind::from_url("https://yukicoder.me/problems/no/1").unwrap();
        assert_eq!(kind, JudgeKind::Yukicoder);
    }

    #[test]
    fn from_url_aoj_problem() {
        let kind = JudgeKind::from_url(
            "https://onlinejudge.u-aizu.ac.jp/problems/ITP1_1_A",
        )
        .unwrap();
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
}
