use axum::{Json, extract::State, response::IntoResponse};

use crate::{
    AppState,
    dto::{VaultListItem, VaultListResponse},
    errors::ApiError,
    helpers::{fetch_vault_portfolio, http_client, map_status},
};
use pragma_db::models::Vault;
use reqwest::StatusCode as HttpStatusCode;

#[utoipa::path(
    get,
    path = "/vaults",
    tag = "Vaults",
    responses(
        (status = 200, description = "Vault list", body = VaultListResponse),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn list_vaults(State(state): State<AppState>) -> Result<impl IntoResponse, ApiError> {
    let conn = state.pool.get().await.map_err(|e| {
        tracing::error!("Failed to get database connection: {}", e);
        ApiError::InternalServerError
    })?;

    let vaults = conn
        .interact(move |conn| Vault::find_all(conn))
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error: {}", e);
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            tracing::error!("Failed to fetch vaults: {}", e);
            ApiError::InternalServerError
        })?;

    // For non-metadata fields (e.g., TVL), query each vault's API endpoint.
    // Keep this resilient: on any failure, default TVL to "0" and continue.
    let client = http_client()?;

    let mut items = Vec::with_capacity(vaults.len());
    for vault in vaults.into_iter() {
        let stats_url = format!("{}/portfolio", vault.api_endpoint.trim_end_matches('/'));

        let tvl = match fetch_vault_portfolio(&client, &vault.api_endpoint).await {
            Some(p) => p.tvl_in_usd,
            None => {
                let code: Option<HttpStatusCode> = None;
                tracing::warn!(vault_id = %vault.id, url = %stats_url, status = ?code, "Failed to fetch vault stats");
                "0".to_string()
            }
        };

        // Map DB status to API spec values: active -> live
        let status = map_status(&vault.status);

        items.push(VaultListItem {
            id: vault.id,
            name: vault.name,
            description: vault.description,
            chain: vault.chain,
            symbol: vault.symbol,
            tvl,
            status,
        });
    }

    Ok(Json(VaultListResponse { items }))
}
