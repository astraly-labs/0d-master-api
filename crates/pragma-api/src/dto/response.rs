#![allow(clippy::option_if_let_else)]

use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum ResponseStatus {
    Ok,
    Error,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[allow(clippy::option_if_let_else)]
pub struct ApiResponse<T> {
    pub status: ResponseStatus,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub msg: Option<String>,
}

impl<T> ApiResponse<T> {
    pub const fn ok(data: T) -> Self {
        Self {
            status: ResponseStatus::Ok,
            data: Some(data),
            msg: None,
        }
    }

    pub const fn error(msg: String) -> Self {
        Self {
            status: ResponseStatus::Error,
            data: None,
            msg: Some(msg),
        }
    }
}
