use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use zerod_master::GetStatsDTO;

use crate::{
    AppState,
    dto::ApiResponse,
    errors::ApiError,
    helpers::{call_vault_backend, fetch_vault_with_client},
};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/stats",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Vault statistics", body = GetStatsDTO),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_stats(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let vault_stats =
        call_vault_backend(&client, &vault, "fetch vault stats", |backend| async move {
            backend.get_vault_stats().await
        })
        .await?;

    Ok(Json(ApiResponse::ok(vault_stats)))
}
