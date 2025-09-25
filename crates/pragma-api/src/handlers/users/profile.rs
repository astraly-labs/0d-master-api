use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    AppState,
    dto::{ApiResponse, UserProfile},
    errors::ApiError,
};
use pragma_db::models::User;

#[utoipa::path(
    get,
    path = "/users/{address}",
    tag = "User",
    params(
        ("address" = String, Path, description = "User wallet address")
    ),
    responses(
        (status = 200, description = "User profile", body = UserProfile),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_user_profile(
    State(state): State<AppState>,
    Path(address): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let conn = state.pool.get().await.map_err(|e| {
        tracing::error!("Failed to get database connection: {}", e);
        ApiError::InternalServerError
    })?;

    let address_clone = address.clone();
    let user = conn
        .interact(move |conn| User::find_by_address(&address_clone, conn))
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error: {}", e);
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            if e == diesel::result::Error::NotFound {
                ApiError::NotFound(format!("User {address} not found"))
            } else {
                tracing::error!("Failed to fetch user: {}", e);
                ApiError::InternalServerError
            }
        })?;

    let profile = UserProfile::from(user);

    Ok(Json(ApiResponse::ok(profile)))
}
