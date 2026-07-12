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
