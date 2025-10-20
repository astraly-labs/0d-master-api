use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use rust_decimal::Decimal;

use crate::{
    AppState,
    dto::{ApiResponse, UserKpi},
    errors::{ApiError, DatabaseErrorExt},
    helpers::validate_indexer_status,
};
use zerod_db::{
    ZerodPool,
    models::{UserPosition, UserTransaction, Vault},
};
use zerod_kpi::calculate_user_pnl;
use zerod_master::JaffarClient;
use zerod_master::VaultMasterClient;

#[utoipa::path(
    get,
    path = "/users/{address}/vaults/{vault_id}/kpis",
    tag = "User",
    params(
        ("address" = String, Path, description = "User wallet address"),
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "User performance KPIs", body = UserKpi),
        (status = 404, description = "User KPIs not found"),
        (status = 503, description = "Indexer not synced or experiencing issues"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_user_kpis(
    State(state): State<AppState>,
    Path((address, vault_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate that the indexer is synced before serving user data
    validate_indexer_status(&vault_id, &state.pool).await?;

    // Run parallel database queries for better performance
    let (position_result, transactions_result, vault_result) = tokio::join!(
        state.pool.interact_with_context(
            format!("find position for user {address} in vault {vault_id}"),
            {
                let address = address.clone();
                let vault_id = vault_id.clone();
                move |conn| UserPosition::find_by_user_and_vault(&address, &vault_id, conn)
            }
        ),
        state.pool.interact_with_context(
            format!("fetch transactions for user {address} in vault {vault_id}"),
            {
                let address = address.clone();
                let vault_id = vault_id.clone();
                move |conn| {
                    UserTransaction::find_by_user_and_vault_chronological(&address, &vault_id, conn)
                }
            }
        ),
        state
            .pool
            .interact_with_context(format!("find vault by id: {vault_id}"), {
                let vault_id = vault_id.clone();
                move |conn| Vault::find_by_id(&vault_id, conn)
            }),
    );

    let position = position_result.map_err(|e| {
        e.or_not_found(format!(
            "Position for user {address} in vault {vault_id} not found"
        ))
    })?;
    let transactions = transactions_result?;
    let vault = vault_result.map_err(|e| e.or_not_found(format!("Vault {vault_id} not found")))?;

    // Fetch current share price from vault API
    let client = JaffarClient::new(&vault.api_endpoint);
    let current_share_price_str = client.get_vault_info().await?.share_price_in_usd;

    let current_share_price = current_share_price_str.parse::<Decimal>().map_err(|e| {
        tracing::error!("Failed to parse share price '{current_share_price_str}': {e}",);
        ApiError::InternalServerError
    })?;

    // Get KPIs from daily calculations (computed by KpiService)
    let address_clone = address.clone();
    let vault_id_clone = vault_id.clone();
    let mut cached_kpis = state
        .pool
        .interact_with_context(
            format!("fetch cached KPIs for user {address} in vault {vault_id}"),
            move |conn| {
                zerod_db::models::UserKpi::find_by_user_and_vault(
                    &address_clone,
                    &vault_id_clone,
                    conn,
                )
            },
        )
        .await?;

    // Calculate PnL metrics real-time for current accuracy
    let pnl_result =
        calculate_user_pnl(&position, &transactions, current_share_price).map_err(|e| {
            tracing::error!("Failed to calculate real-time PnL: {e}");
            ApiError::InternalServerError
        })?;

    cached_kpis.all_time_pnl = Some(pnl_result.all_time_pnl);
    cached_kpis.unrealized_pnl = Some(pnl_result.unrealized_pnl);
    cached_kpis.realized_pnl = Some(pnl_result.realized_pnl);

    Ok(Json(ApiResponse::ok(cached_kpis)))
}
