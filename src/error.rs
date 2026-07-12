use thiserror::Error;

#[derive(Debug, Error)]
pub enum AppError {
    #[error("HTTP error: {0}")]
    Http(#[from] reqwest::Error),

    #[error("IO error: {0}")]
    Io(#[from] std::io::Error),

    #[error("JSON error: {0}")]
    Json(#[from] serde_json::Error),

    #[error("TOML parse error: {0}")]
    TomlDe(#[from] toml::de::Error),

    #[error("TOML serialize error: {0}")]
    TomlSe(#[from] toml::ser::Error),

    /// URL がどのジャッジにも一致しない
    #[error("Unsupported URL: {0}")]
    UnsupportedUrl(String),

    /// 問題ページからサンプルが取得できなかった
    #[error("Failed to parse samples: {0}")]
    SampleParse(String),

    /// メタデータファイルが見つからない
    #[error("No metadata found. Run `je new` or `je add` first.")]
    MetaNotFound,

    /// OS 標準ディレクトリの取得に失敗
    #[error("Could not determine config directory")]
    ConfigDirNotFound,
}
