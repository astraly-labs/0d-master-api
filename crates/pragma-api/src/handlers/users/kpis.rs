use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use chrono::Utc;
use rust_decimal::Decimal;

use crate::{
    AppState,
    dto::UserKpi,
    errors::ApiError,
    helpers::{fetch_vault_share_price, http_client},
};
use pragma_db::models::{UserPosition, UserTransaction, Vault};
use pragma_kpi::calculate_user_kpis;

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
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_user_kpis(
    State(state): State<AppState>,
    Path((address, vault_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
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
    let client = http_client()?;
    let current_share_price_str = fetch_vault_share_price(&client, &vault.api_endpoint)
        .await
        .ok_or_else(|| {
            tracing::error!("Failed to fetch current share price from vault API");
            ApiError::InternalServerError
        })?;

    let current_share_price = current_share_price_str.parse::<Decimal>().map_err(|e| {
        tracing::error!("Failed to parse share price '{current_share_price_str}': {e}",);
        ApiError::InternalServerError
    })?;

    // // Get risk metrics from daily calculations (computed by KpiService)
    // let cached_risk_metrics = conn
    //     .interact({
    //         let address = address.clone();
    //         let vault_id = vault_id.clone();
    //         move |conn| {
    //             pragma_db::models::UserKpi::find_by_user_and_vault(&address, &vault_id, conn)
    //         }
    //     })
    //     .await;

    // Calculate PnL metrics real-time for current accuracy
    let kpi_result =
        calculate_user_kpis(&position, &transactions, current_share_price).map_err(|e| {
            tracing::error!("Failed to calculate real-time PnL: {e}");
            ApiError::InternalServerError
        })?;

    let user_kpi = UserKpi {
        as_of: Utc::now(),
        // Real-time PnL calculations (accurate to current share price)
        all_time_pnl_usd: kpi_result.all_time_pnl.to_string(),
        unrealized_pnl_usd: kpi_result.unrealized_pnl.to_string(),
        realized_pnl_usd: kpi_result.realized_pnl.to_string(),
        // Daily computed risk metrics (from KpiService)
        max_drawdown_pct: 0.0,
        sharpe: 0.0,
        sortino: 0.0,
    };

    Ok(Json(user_kpi))
}
