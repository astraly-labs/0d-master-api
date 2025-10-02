use axum::{Json, extract::State, response::IntoResponse};
use futures::future::join_all;

use crate::{
    AppState,
    dto::{ApiResponse, VaultListItem, VaultListResponse},
    errors::ApiError,
    helpers::{is_alternative_vault, map_status},
};
use zerod_db::{ZerodPool, models::Vault};
use zerod_master::{VaultAlternativeAPIClient, VaultMasterAPIClient};

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
    let vaults = state
        .pool
        .interact_with_context("fetch all vaults".to_string(), Vault::find_all)
        .await?;

    // Fetch vault stats from external APIs in parallel
    let fetch_futures = vaults.iter().map(|vault| {
        let vault = vault.clone();
        async move {
            let (tvl, apr, average_redeem_delay, last_reported) = if is_alternative_vault(&vault.id) {
                match VaultAlternativeAPIClient::new(&vault.api_endpoint, &vault.contract_address) {
                    Ok(client) => match client.get_vault().await {
                        Ok(data) => {
                            let tvl = data.tvl.unwrap_or_else(|| "0".to_string());
                            let apr = data.apr.unwrap_or_else(|| "0".to_string());
                            (tvl, apr, data.average_redeem_delay, data.last_reported)
                        }
                        Err(err) => {
                            tracing::warn!(
                                vault_id = %vault.id,
                                error = %err,
                                "Failed to fetch alternative vault snapshot"
                            );
                            ("0".to_string(), "0".to_string(), None, None)
                        }
                    },
                    Err(err) => {
                        tracing::warn!(
                            vault_id = %vault.id,
                            error = %err,
                            "Failed to create alternative vault client"
                        );
                        ("0".to_string(), "0".to_string(), None, None)
                    }
                }
            } else {
                match VaultMasterAPIClient::new(&vault.api_endpoint) {
                    Ok(client) => match client.get_vault_stats().await {
                        Ok(p) => (p.tvl, p.past_month_apr_pct.to_string(), None, None),
                        Err(err) => {
                            tracing::warn!(vault_id = %vault.id, error = %err, "Failed to fetch vault stats");
                            ("0".to_string(), "0".to_string(), None, None)
                        }
                    },
                    Err(err) => {
                        tracing::warn!(vault_id = %vault.id, error = %err, "Failed to create vault client");
                        ("0".to_string(), "0".to_string(), None, None)
                    }
                }
            };

            let status = map_status(&vault.status);

            VaultListItem {
                id: vault.id,
                name: vault.name,
                description: vault.description,
                chain: vault.chain,
                symbol: vault.symbol,
                tvl,
                apr,
                status,
                average_redeem_delay,
                last_reported,
            }
        }
    });

    let items = join_all(fetch_futures).await;

    Ok(Json(ApiResponse::ok(VaultListResponse { items })))
}
