use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    AppState,
    dto::{
        DisplayCurrency, HistoricalDataPoint, HistoricalUserPerformance, PerformanceMetric,
        Timeframe,
    },
    errors::ApiError,
};
use pragma_db::models::UserKpi;

#[derive(Debug, Deserialize)]
pub struct HistoricalQuery {
    pub metric: PerformanceMetric,
    #[serde(default = "default_timeframe")]
    pub timeframe: Timeframe,
    #[serde(default = "default_currency")]
    pub currency: DisplayCurrency,
}

fn default_timeframe() -> Timeframe {
    Timeframe::All
}

fn default_currency() -> DisplayCurrency {
    DisplayCurrency::USD
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
        ("currency" = Option<DisplayCurrency>, Query, description = "Display currency")
    ),
    responses(
        (status = 200, description = "Historical User performance", body = HistoricalUserPerformance),
        (status = 404, description = "User or vault not found"),
        (status = 400, description = "Invalid parameters"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_historical_user_performance(
    State(state): State<AppState>,
    Path((address, vault_id)): Path<(String, String)>,
    Query(query): Query<HistoricalQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let conn = state.pool.get().await.map_err(|e| {
        tracing::error!("Failed to get database connection: {}", e);
        ApiError::InternalServerError
    })?;

    // Validate that the vault exists first
    let vault_exists = conn
        .interact({
            let vault_id = vault_id.clone();
            move |conn| {
                use pragma_db::models::Vault;
                Vault::find_by_id(&vault_id, conn).is_ok()
            }
        })
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error: {e}");
            ApiError::InternalServerError
        })?;

    if !vault_exists {
        return Err(ApiError::NotFound(format!("Vault {vault_id} not found")));
    }

    // Get historical performance data
    let historical_data = conn
        .interact({
            let address = address.clone();
            let vault_id = vault_id.clone();
            let metric = query.metric.clone();
            let timeframe = query.timeframe.clone();
            move |conn| {
                UserKpi::get_historical_performance(&address, &vault_id, &metric, &timeframe, conn)
            }
        })
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error: {e}");
            ApiError::InternalServerError
        })?;

    let historical_data = historical_data.map_err(|e| {
        tracing::error!("Failed to fetch historical data: {e}");
        ApiError::InternalServerError
    })?;

    // Convert to API format
    let points: Vec<HistoricalDataPoint> = historical_data
        .into_iter()
        .map(|(timestamp, value)| HistoricalDataPoint {
            t: timestamp,
            v: value.to_string(),
        })
        .collect();

    let response = HistoricalUserPerformance {
        metric: query.metric,
        timeframe: query.timeframe,
        points,
    };

    Ok(Json(response))
}
