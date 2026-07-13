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

    /// API が非成功ステータスを返した
    #[error("API error {status} for {url}: {body}")]
    ApiError {
        status: u16,
        url: String,
        body: String,
    },

    /// 問題ページからサンプルが取得できなかった
    #[error("Failed to parse samples: {0}")]
    SampleParse(String),

    /// メタデータファイルが見つからない
    #[error("No metadata found. Run `je prepare` first.")]
    MetaNotFound,

    /// OS 標準ディレクトリの取得に失敗
    #[error("Could not determine config directory")]
    ConfigDirNotFound,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unsupported_url_display() {
        let err = AppError::UnsupportedUrl("https://example.com".to_string());
        assert_eq!(err.to_string(), "Unsupported URL: https://example.com");
    }

    #[test]
    fn sample_parse_display() {
        let err = AppError::SampleParse("no samples found".to_string());
        assert_eq!(err.to_string(), "Failed to parse samples: no samples found");
    }

    #[test]
    fn meta_not_found_display() {
        let err = AppError::MetaNotFound;
        assert_eq!(
            err.to_string(),
            "No metadata found. Run `je prepare` first."
        );
    }

    #[test]
    fn config_dir_not_found_display() {
        let err = AppError::ConfigDirNotFound;
        assert_eq!(err.to_string(), "Could not determine config directory");
    }

    #[test]
    fn api_error_display() {
        let err = AppError::ApiError {
            status: 404,
            url: "https://yukicoder.me/api/v1/contest/id/9999".to_string(),
            body: "Not Found".to_string(),
        };
        assert_eq!(
            err.to_string(),
            "API error 404 for https://yukicoder.me/api/v1/contest/id/9999: Not Found"
        );
    }

    #[test]
    fn io_error_display() {
        let io_err = std::io::Error::new(std::io::ErrorKind::NotFound, "file not found");
        let err = AppError::Io(io_err);
        assert!(err.to_string().starts_with("IO error:"));
    }

    #[test]
    fn json_error_display() {
        let json_err = serde_json::from_str::<serde_json::Value>("invalid json")
            .unwrap_err();
        let err = AppError::Json(json_err);
        assert!(err.to_string().starts_with("JSON error:"));
    }
}
