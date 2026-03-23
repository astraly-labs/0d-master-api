use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use chrono::Utc;
use serde::Deserialize;

use zerod_db::ZerodPool;
use zerod_db::models::UserPortfolioHistory;
use zerod_db::types::Timeframe;
use zerod_master::{TimeseriesPoint, TimeseriesResponseDTO};

use crate::{AppState, dto::ApiResponse, errors::ApiError};

#[derive(Debug, Deserialize)]
pub struct SharePriceSeriesQuery {
    #[serde(default)]
    pub timeframe: Timeframe,
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/share-price/series",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("timeframe" = Option<Timeframe>, Query, description = "Time period", example = "all")
    ),
    responses(
        (status = 200, description = "Share price time series", body = TimeseriesResponseDTO),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_share_price_series(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
    Query(params): Query<SharePriceSeriesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let since = params
        .timeframe
        .to_days()
        .map(|days| Utc::now() - chrono::Duration::days(days));

    let vault_id_clone = vault_id.clone();
    let points = state
        .pool
        .interact_with_context(
            format!("fetch share price series for vault: {vault_id}"),
            move |conn| UserPortfolioHistory::get_share_price_series(&vault_id_clone, since, conn),
        )
        .await?;

    let response = TimeseriesResponseDTO {
        metric: "share_price".to_string(),
        timeframe: params.timeframe.as_str().to_string(),
        points: points
            .into_iter()
            .map(|(ts, price)| TimeseriesPoint {
                t: ts.to_rfc3339(),
                v: price.to_string(),
            })
            .collect(),
    };

    Ok(Json(ApiResponse::ok(response)))
}
