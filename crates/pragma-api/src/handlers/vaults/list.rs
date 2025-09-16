use axum::{
    extract::State,
    response::IntoResponse,
    Json,
};

use crate::{
    dto::{ApiResponse, VaultListItem, VaultListResponse, VaultStats},
    errors::ApiError,
    AppState,
};
use pragma_db::models::Vault;
use reqwest::StatusCode as HttpStatusCode;
use std::time::Duration;

#[utoipa::path(
    get,
    path = "/vaults",
    tag = "Vaults",
    responses(
        (status = 200, description = "Vault list", body = ApiResponse<VaultListResponse>),
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
    let client = reqwest::Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| {
            tracing::error!("Failed to build HTTP client: {}", e);
            ApiError::InternalServerError
        })?;

    let mut items = Vec::with_capacity(vaults.len());
    for vault in vaults.into_iter() {
        let stats_url = format!(
            "{}/stats",
            vault.api_endpoint.trim_end_matches('/')
        );

        let tvl = match client.get(&stats_url).send().await {
            Ok(resp) => {
                // Treat non-2xx as failure
                if !resp.status().is_success() {
                    tracing::warn!(
                        vault_id = %vault.id,
                        status = %resp.status().as_u16(),
                        url = %stats_url,
                        "Vault stats request returned non-success status"
                    );
                    "0".to_string()
                } else {
                    match resp.json::<VaultStats>().await {
                        Ok(stats) => stats.tvl,
                        Err(e) => {
                            tracing::warn!(
                                vault_id = %vault.id,
                                error = %e,
                                url = %stats_url,
                                "Failed to parse vault stats JSON"
                            );
                            "0".to_string()
                        }
                    }
                }
            }
            Err(e) => {
                // Network/timeout error; log and fallback
                let code: Option<HttpStatusCode> = e.status();
                tracing::warn!(
                    vault_id = %vault.id,
                    url = %stats_url,
                    status = ?code.map(|c| c.as_u16()),
                    error = %e,
                    "Failed to fetch vault stats"
                );
                "0".to_string()
            }
        };

        items.push(VaultListItem {
            id: vault.id,
            name: vault.name,
            description: vault.description,
            chain: vault.chain,
            symbol: vault.symbol,
            tvl,
            status: vault.status,
        });
    }

    Ok(Json(ApiResponse::ok(VaultListResponse { items })))
}
