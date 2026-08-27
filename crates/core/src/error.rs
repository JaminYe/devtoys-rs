use thiserror::Error;

#[derive(Debug, Error)]
pub enum CoreError {
    #[error("unknown tool '{0}'")]
    UnknownTool(String),
    #[error("tool '{0}' cannot be favorited")]
    NotFavorable(String),
    #[error("failed to write settings: {0}")]
    Io(#[from] std::io::Error),
    #[error("failed to serialize settings: {0}")]
    Json(#[from] serde_json::Error),
}
