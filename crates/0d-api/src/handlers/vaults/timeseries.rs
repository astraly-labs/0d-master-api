use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use zerod_db::types::{Currency, Metric, Timeframe};
use zerod_master::TimeseriesResponseDTO;

use crate::{
    AppState,
    dto::{ApiResponse, TimeseriesQuery},
    errors::ApiError,
    helpers::{call_vault_backend, fetch_vault_with_client},
};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/timeseries",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("metric" = Metric, Query, description = "Vault-level metric to return", example = "tvl"),
        ("timeframe" = Timeframe, Query, description = "Time period", example = "7d"),
        ("currency" = Currency, Query, description = "Currency for values", example = "USD")
    ),
    responses(
        (status = 200, description = "Vault timeseries data", body = TimeseriesResponseDTO),
        (status = 400, description = "Invalid parameters"),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_timeseries(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
    Query(params): Query<TimeseriesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let metric = params.metric.as_str().to_owned();
    let timeframe = params.timeframe.as_str().to_owned();
    let currency = params.currency.to_string();
    let timeseries =
        call_vault_backend(&client, &vault, "fetch vault timeseries", move |backend| {
            let metric = metric.clone();
            let timeframe = timeframe.clone();
            let currency = currency.clone();
            async move {
                backend
                    .get_vault_timeseries(&metric, &timeframe, &currency)
                    .await
            }
        })
        .await?;

    Ok(Json(ApiResponse::ok(timeseries)))
}
