use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::helpers::{is_alternative_vault, map_status};
use crate::{
    AppState,
    dto::{ApiResponse, Vault as VaultDto},
    errors::{ApiError, DatabaseErrorExt},
};
use zerod_db::{ZerodPool, models::Vault};
use zerod_master::{VaultAlternativeAPIClient, VaultMasterAPIClient};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Vault metadata with live values", body = VaultDto),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault(
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

    // Fetch non-metadata (tvl & share_price) from the vault's API endpoint.
    let (vault_stats, share_price) = if is_alternative_vault(&vault.id) {
        let client = VaultAlternativeAPIClient::new(&vault.api_endpoint, &vault.contract_address)?;
        let vault_stats = client.get_vault_stats().await.map_err(|e| {
            tracing::error!("Failed to fetch alternative vault stats: {e}");
            ApiError::InternalServerError
        })?;
        let share_price = client.get_vault_share_price().await.map_err(|e| {
            tracing::error!("Failed to fetch alternative vault share price: {e}");
            ApiError::InternalServerError
        })?;
        (vault_stats, share_price)
    } else {
        let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
        let vault_stats = client.get_vault_stats().await.map_err(|e| {
            tracing::error!("Failed to fetch vault stats: {e}");
            ApiError::InternalServerError
        })?;
        let share_price = client.get_vault_share_price().await.map_err(|e| {
            tracing::error!("Failed to fetch vault share price: {e}");
            ApiError::InternalServerError
        })?;
        (vault_stats, share_price)
    };

    // Build response DTO and embed live values.
    let mut dto = VaultDto::from(vault);
    // Map status to spec: active -> live
    dto.status = map_status(&dto.status);
    dto.tvl = vault_stats.tvl;
    dto.share_price = share_price;

    Ok(Json(ApiResponse::ok(dto)))
}
