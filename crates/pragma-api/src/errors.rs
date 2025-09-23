use crate::dto::ApiResponse;
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Error, Debug, Serialize, Deserialize)]
pub enum ApiError {
    #[error("Database error: {0}")]
    DbError(String),
    #[error("Not implemented: {0}")]
    NotImplemented(String),
    #[error("Unauthorized: {0}")]
    Unauthorized(String),
    #[error("Not found: {0}")]
    NotFound(String),
    #[error("Bad request: {0}")]
    BadRequest(String),
    #[error("Internal server error")]
    InternalServerError,
}

impl From<pragma_master::MasterApiError> for ApiError {
    fn from(err: pragma_master::MasterApiError) -> Self {
        match err {
            pragma_master::MasterApiError::InternalServerError
            | pragma_master::MasterApiError::HttpError(_)
            | pragma_master::MasterApiError::JsonError(_)
            | pragma_master::MasterApiError::AnyhowError(_) => Self::InternalServerError,
        }
    }
}

impl IntoResponse for ApiError {
    fn into_response(self) -> axum::response::Response {
        let (status, msg) = match self {
            Self::Unauthorized(msg) => (StatusCode::UNAUTHORIZED, msg),
            Self::DbError(msg) => (StatusCode::INTERNAL_SERVER_ERROR, msg),
            Self::NotImplemented(msg) => (StatusCode::NOT_IMPLEMENTED, msg),
            Self::NotFound(msg) => (StatusCode::NOT_FOUND, msg),
            Self::BadRequest(msg) => (StatusCode::BAD_REQUEST, msg),
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };
        let response: ApiResponse<()> = ApiResponse::error(msg);
        (status, Json(response)).into_response()
    }
}
