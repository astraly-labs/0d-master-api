use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use pragma_db::models::Vault;
use pragma_master::{GetStatsDTO, VaultMasterAPIClient};

use crate::{AppState, dto::ApiResponse, errors::ApiError};

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
    // Find the vault to get its API endpoint
    let conn = state.pool.get().await.map_err(|e| {
        tracing::error!("Failed to get database connection: {}", e);
        ApiError::InternalServerError
    })?;

    let vault_id_clone = vault_id.clone();
    let vault = conn
        .interact(move |conn| Vault::find_by_id(&vault_id_clone, conn))
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error: {}", e);
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            if e == diesel::result::Error::NotFound {
                ApiError::NotFound(format!("Vault {vault_id} not found"))
            } else {
                tracing::error!("Failed to fetch vault: {}", e);
                ApiError::InternalServerError
            }
        })?;

    // Call the vault's portfolio/stats endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let vault_stats = client.get_vault_stats().await.map_err(|e| {
        tracing::error!("Failed to fetch vault stats: {}", e);
        ApiError::InternalServerError
    })?;

    Ok(Json(ApiResponse::ok(vault_stats)))
}
