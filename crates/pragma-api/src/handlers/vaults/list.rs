use axum::{Json, extract::State, response::IntoResponse};

use crate::{
    AppState,
    dto::{ApiResponse, VaultListItem, VaultListResponse},
    errors::ApiError,
    helpers::{is_alternative_vault, map_status},
};
use pragma_db::models::Vault;
use pragma_master::{VaultAlternativeAPIClient, VaultMasterAPIClient};
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
        .interact(Vault::find_all)
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

    let mut items = Vec::with_capacity(vaults.len());
    for vault in vaults {
        let tvl = if is_alternative_vault(&vault.id) {
            let client =
                VaultAlternativeAPIClient::new(&vault.api_endpoint, &vault.contract_address)?;
            match client.get_vault_stats().await {
                Ok(stats) => stats.tvl,
                Err(err) => {
                    tracing::warn!(vault_id = %vault.id, error = %err, "Failed to fetch alternative vault stats");
                    "0".to_string()
                }
            }
        } else {
            let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
            if let Ok(p) = client.get_vault_stats().await {
                p.tvl
            } else {
                let code: Option<HttpStatusCode> = None;
                tracing::warn!(vault_id = %vault.id, status = ?code, "Failed to fetch vault stats");
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

    Ok(Json(ApiResponse::ok(VaultListResponse { items })))
}
