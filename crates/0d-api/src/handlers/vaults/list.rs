use axum::{Json, extract::State, response::IntoResponse};
use futures::future::try_join_all;

use crate::{
    AppState,
    dto::{ApiResponse, VaultListItem, VaultListResponse},
    errors::ApiError,
    helpers::{VaultBackendClient, call_vault_backend, map_status},
};
use zerod_db::{ZerodPool, models::Vault};

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

    let fetch_futures = vaults.into_iter().map(|vault| async move {
        let client = VaultBackendClient::new(&vault)?;
        let stats =
            call_vault_backend(&client, &vault, "fetch vault stats", |backend| async move {
                backend.get_vault_stats().await
            })
            .await?;

        Ok::<_, ApiError>(VaultListItem {
            id: vault.id,
            name: vault.name,
            description: vault.description,
            chain: vault.chain,
            symbol: vault.symbol,
            tvl: stats.tvl,
            apr: stats.past_month_apr_pct.to_string(),
            status: map_status(&vault.status),
            average_redeem_delay: None,
            last_reported: None,
        })
    });

    let items = try_join_all(fetch_futures).await?;

    Ok(Json(ApiResponse::ok(VaultListResponse { items })))
}
