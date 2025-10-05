use crate::dto::ApiResponse;
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::{Deserialize, Serialize};
use thiserror::Error;
use zerod_db::DatabaseError;
use zerod_master::MasterApiError;

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
    #[error("Service unavailable: {0}")]
    ServiceUnavailable(String),
    #[error("Internal server error")]
    InternalServerError,
}

impl From<DatabaseError> for ApiError {
    fn from(err: DatabaseError) -> Self {
        // NOTE: Error is already logged in the DatabaseError layer
        match err {
            DatabaseError::NotFound { .. } => {
                Self::NotFound("The requested resource was not found".to_string())
            }
            DatabaseError::PoolError { .. }
            | DatabaseError::InteractionError { .. }
            | DatabaseError::QueryError { .. }
            | DatabaseError::UniqueViolation { .. }
            | DatabaseError::ForeignKeyViolation { .. } => {
                // Don't expose internal database details to clients
                Self::InternalServerError
            }
        }
    }
}

/// Extension trait for `DatabaseError` to provide convenient conversion to `ApiError`
pub trait DatabaseErrorExt {
    /// Convert to `ApiError` with a custom `NotFound` message, or use default conversion
    fn or_not_found(self, message: String) -> ApiError;
}

impl DatabaseErrorExt for DatabaseError {
    fn or_not_found(self, message: String) -> ApiError {
        if self.is_not_found() {
            ApiError::NotFound(message)
        } else {
            self.into()
        }
    }
}

impl From<MasterApiError> for ApiError {
    fn from(err: MasterApiError) -> Self {
        match err {
            MasterApiError::InternalServerError
            | MasterApiError::HttpError(_)
            | MasterApiError::JsonError(_)
            | MasterApiError::AnyhowError(_)
            | MasterApiError::JaffarSdkError(_)
            | MasterApiError::VesuSdkError(_) => Self::InternalServerError,
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
            Self::ServiceUnavailable(msg) => (StatusCode::SERVICE_UNAVAILABLE, msg),
            Self::InternalServerError => (
                StatusCode::INTERNAL_SERVER_ERROR,
                "Internal server error".to_string(),
            ),
        };
        let response: ApiResponse<()> = ApiResponse::error(msg);
        (status, Json(response)).into_response()
    }
}
