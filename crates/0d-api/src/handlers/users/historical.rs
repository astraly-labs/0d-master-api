use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    AppState,
    dto::ApiResponse,
    dto::{HistoricalDataPoint, HistoricalUserPerformance, PerformanceMetric, Timeframe},
    errors::{ApiError, DatabaseErrorExt},
    helpers::{quote_to_currency, validate_indexer_status},
};
use zerod_db::{ZerodPool, models::UserKpi};
use zerod_quoting::currencies::Currency;

#[derive(Debug, Deserialize)]
pub struct HistoricalQuery {
    pub metric: PerformanceMetric,
    #[serde(default = "default_timeframe")]
    pub timeframe: Timeframe,
    #[serde(default = "default_currency")]
    pub currency: Currency,
}

const fn default_timeframe() -> Timeframe {
    Timeframe::All
}

const fn default_currency() -> Currency {
    Currency::USD
}

#[utoipa::path(
    get,
    path = "/users/{address}/vaults/{vault_id}/historical",
    tag = "User",
    params(
        ("address" = String, Path, description = "User wallet address"),
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("metric" = PerformanceMetric, Query, description = "User performance metric to return"),
        ("timeframe" = Option<Timeframe>, Query, description = "Time period for data"),
        ("currency" = Option<Currency>, Query, description = "Display currency")
    ),
    responses(
        (status = 200, description = "Historical User performance", body = HistoricalUserPerformance),
        (status = 404, description = "User or vault not found"),
        (status = 400, description = "Invalid parameters"),
        (status = 503, description = "Indexer not synced or experiencing issues"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_historical_user_performance(
    State(state): State<AppState>,
    Path((address, vault_id)): Path<(String, String)>,
    Query(query): Query<HistoricalQuery>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate that the indexer is synced before serving user data
    validate_indexer_status(&vault_id, &state.pool).await?;

    // Validate that the vault exists first
    let vault_id_clone = vault_id.clone();
    let _ = state
        .pool
        .interact_with_context(format!("find vault by id: {vault_id}"), move |conn| {
            use zerod_db::models::Vault;
            Vault::find_by_id(&vault_id_clone, conn)
        })
        .await
        .map_err(|e| e.or_not_found(format!("Vault {vault_id} not found")))?;

    // Get historical performance data
    let address_clone = address.clone();
    let vault_id_clone = vault_id.clone();
    let metric = query.metric.clone();
    let timeframe = query.timeframe.clone();
    let historical_data = state
        .pool
        .interact_with_context(
            format!("fetch historical performance for user {address} in vault {vault_id}"),
            move |conn| {
                UserKpi::get_historical_performance(
                    &address_clone,
                    &vault_id_clone,
                    &metric,
                    &timeframe,
                    conn,
                )
            },
        )
        .await?;

    // Convert to API format with currency conversion
    let mut points: Vec<HistoricalDataPoint> = Vec::new();
    for (timestamp, value) in historical_data {
        let converted_value = quote_to_currency(value, query.currency).await?;
        points.push(HistoricalDataPoint {
            t: timestamp,
            v: converted_value.to_string(),
        });
    }

    let response = HistoricalUserPerformance {
        metric: query.metric,
        timeframe: query.timeframe,
        points,
    };

    Ok(Json(ApiResponse::ok(response)))
}
