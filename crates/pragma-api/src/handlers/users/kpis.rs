use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use rust_decimal::Decimal;

use crate::{AppState, dto::UserKpi, errors::ApiError, helpers::validate_indexer_status};
use pragma_db::models::{UserPosition, UserTransaction, Vault};
use pragma_kpi::calculate_user_pnl;
use pragma_master::VaultMasterAPIClient;

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

    let conn = state.pool.get().await.map_err(|e| {
        tracing::error!("Failed to get database connection: {}", e);
        ApiError::InternalServerError
    })?;

    // Get user position and transactions
    let address_clone = address.clone();
    let vault_id_clone = vault_id.clone();

    let (position_result, transactions_result, vault_result) = tokio::join!(
        conn.interact({
            let address = address_clone.clone();
            let vault_id = vault_id_clone.clone();
            move |conn| UserPosition::find_by_user_and_vault(&address, &vault_id, conn)
        }),
        conn.interact({
            let address = address_clone.clone();
            let vault_id = vault_id_clone.clone();
            move |conn| {
                UserTransaction::find_by_user_and_vault_chronological(&address, &vault_id, conn)
            }
        }),
        conn.interact({
            let vault_id = vault_id_clone.clone();
            move |conn| Vault::find_by_id(&vault_id, conn)
        })
    );

    // Handle database interaction errors
    let position = position_result
        .map_err(|e| {
            tracing::error!("Database interaction error for position: {e}");
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            if e == diesel::result::Error::NotFound {
                ApiError::NotFound(format!(
                    "Position for user {address} in vault {vault_id} not found"
                ))
            } else {
                tracing::error!("Failed to fetch user position: {e}");
                ApiError::InternalServerError
            }
        })?;

    let transactions = transactions_result
        .map_err(|e| {
            tracing::error!("Database interaction error for transactions: {e}");
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            tracing::error!("Failed to fetch user transactions: {e}");
            ApiError::InternalServerError
        })?;

    let vault = vault_result
        .map_err(|e| {
            tracing::error!("Database interaction error for vault: {e}");
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            if e == diesel::result::Error::NotFound {
                ApiError::NotFound(format!("Vault {vault_id} not found"))
            } else {
                tracing::error!("Failed to fetch vault: {e}");
                ApiError::InternalServerError
            }
        })?;

    // Fetch current share price from vault API
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let current_share_price_str = client.get_vault_share_price().await.map_err(|e| {
        tracing::error!("Failed to fetch vault share price: {}", e);
        ApiError::InternalServerError
    })?;

    let current_share_price = current_share_price_str.parse::<Decimal>().map_err(|e| {
        tracing::error!("Failed to parse share price '{current_share_price_str}': {e}",);
        ApiError::InternalServerError
    })?;

    // // Get KPIs from daily calculations (computed by KpiService)
    let mut cached_kpis = conn
        .interact({
            let address = address.clone();
            let vault_id = vault_id.clone();
            move |conn| {
                pragma_db::models::UserKpi::find_by_user_and_vault(&address, &vault_id, conn)
            }
        })
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error for risk metrics: {e}");
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            tracing::error!("Failed to fetch KPIs: {e}");
            ApiError::InternalServerError
        })?;

    // Calculate PnL metrics real-time for current accuracy
    let pnl_result =
        calculate_user_pnl(&position, &transactions, current_share_price).map_err(|e| {
            tracing::error!("Failed to calculate real-time PnL: {e}");
            ApiError::InternalServerError
        })?;

    cached_kpis.all_time_pnl = Some(pnl_result.all_time_pnl);
    cached_kpis.unrealized_pnl = Some(pnl_result.unrealized_pnl);
    cached_kpis.realized_pnl = Some(pnl_result.realized_pnl);

    Ok(Json(cached_kpis))
}
