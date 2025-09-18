use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    AppState,
    dto::VaultStats,
    errors::ApiError,
    helpers::{fetch_vault_stats, http_client},
};
use pragma_db::models::Vault;

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/stats",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Vault statistics", body = VaultStats),
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

    // Call the vault's stats endpoint via helper
    let client = http_client()?;
    let fetched_stats = fetch_vault_stats(&client, &vault.api_endpoint).await;
    let tvl = fetched_stats
        .as_ref()
        .map_or_else(|| "0".to_string(), |s| s.tvl.clone());
    let apr = fetched_stats
        .and_then(|s| s.past_month_apr_pct)
        .unwrap_or(0.0);

    Ok(Json(VaultStats {
        tvl,
        past_month_apr_pct: apr,
    }))
}
