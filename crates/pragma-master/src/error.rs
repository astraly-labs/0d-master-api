use thiserror::Error;

#[derive(Error, Debug)]
pub enum MasterApiError {
    #[error("Internal server error")]
    InternalServerError,

    #[error("HTTP request failed: {0}")]
    HttpError(#[from] reqwest::Error),

    #[error("JSON deserialization failed: {0}")]
    JsonError(#[from] serde_json::Error),

    #[error("Anyhow error: {0}")]
    AnyhowError(#[from] anyhow::Error),
}
