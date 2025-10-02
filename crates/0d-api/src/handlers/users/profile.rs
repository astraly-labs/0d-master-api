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
use zerod_db::{ZerodPool, models::User};

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
    let address_clone = address.clone();
    let user = state
        .pool
        .interact_with_context(format!("find user by address: {address}"), move |conn| {
            User::find_by_address(&address_clone, conn)
        })
        .await
        .map_err(|e| {
            if e.is_not_found() {
                return ApiError::NotFound(format!("User {address} not found"));
            }
            e.into()
        })?;

    let profile = UserProfile::from(user);

    Ok(Json(ApiResponse::ok(profile)))
}
