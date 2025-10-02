use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use zerod_db::{ZerodPool, models::Vault};
use zerod_master::{GetStatsDTO, VaultAlternativeAPIClient, VaultMasterAPIClient};

use crate::{
    AppState,
    dto::ApiResponse,
    errors::{ApiError, DatabaseErrorExt},
    helpers::is_alternative_vault,
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
    let vault_id_clone = vault_id.clone();
    let vault = state
        .pool
        .interact_with_context(format!("find vault by id: {vault_id}"), move |conn| {
            Vault::find_by_id(&vault_id_clone, conn)
        })
        .await
        .map_err(|e| e.or_not_found(format!("Vault {vault_id} not found")))?;

    // Call the vault's portfolio/stats endpoint via helper
    let vault_stats = if is_alternative_vault(&vault.id) {
        let client = VaultAlternativeAPIClient::new(&vault.api_endpoint, &vault.contract_address)?;
        client.get_vault_stats().await.map_err(|e| {
            tracing::error!("Failed to fetch alternative vault stats: {}", e);
            ApiError::InternalServerError
        })?
    } else {
        let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
        client.get_vault_stats().await.map_err(|e| {
            tracing::error!("Failed to fetch vault stats: {}", e);
            ApiError::InternalServerError
        })?
    };

    Ok(Json(ApiResponse::ok(vault_stats)))
}
