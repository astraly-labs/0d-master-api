use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::helpers::{fetch_vault_share_price, fetch_vault_stats, http_client, map_status};
use crate::{AppState, dto::Vault as VaultDto, errors::ApiError};
use pragma_db::models::Vault;

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

    // Fetch non-metadata (tvl & share_price) from the vault's API endpoint.
    let client = http_client()?;

    let tvl = fetch_vault_stats(&client, &vault.api_endpoint)
        .await
        .map_or_else(|| "0".to_string(), |s| s.tvl);

    let share_price = fetch_vault_share_price(&client, &vault.api_endpoint)
        .await
        .unwrap_or_else(|| "0".to_string());

    // Build response DTO and embed live values.
    let mut dto = VaultDto::from(vault);
    // Map status to spec: active -> live
    dto.status = map_status(&dto.status);
    dto.tvl = tvl;
    dto.share_price = share_price;

    Ok(Json(dto))
}
