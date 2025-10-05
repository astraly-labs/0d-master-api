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

    #[error("Jaffar SDK error: {0}")]
    JaffarSdkError(String),

    #[error("Vesu SDK error: {0}")]
    VesuSdkError(String),
}

impl<T> From<jaffar_sdk::Error<T>> for MasterApiError
where
    T: std::fmt::Debug,
{
    fn from(err: jaffar_sdk::Error<T>) -> Self {
        MasterApiError::JaffarSdkError(format!("{:?}", err))
    }
}
