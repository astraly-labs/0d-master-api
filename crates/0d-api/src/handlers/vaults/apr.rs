use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use zerod_db::types::{AprBasis, Timeframe};
use zerod_master::{AprSeriesDTO, AprSummaryDTO};

use crate::{
    AppState,
    dto::{ApiResponse, AprSeriesQuery, AprSummaryQuery},
    errors::ApiError,
    helpers::{call_vault_backend, fetch_vault_with_client},
};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/apr/summary",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("apr_basis" = AprBasis, Query, description = "APR calculation basis", example = "nominal")
    ),
    responses(
        (status = 200, description = "APR summary", body = AprSummaryDTO),
        (status = 400, description = "Invalid parameters"),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_apr_summary(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
    Query(params): Query<AprSummaryQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let apr_basis = params.apr_basis.as_str().to_owned();
    let apr_summary = call_vault_backend(&client, &vault, "fetch APR summary", move |backend| {
        let apr_basis = apr_basis.clone();
        async move { backend.get_vault_apr_summary(&apr_basis).await }
    })
    .await?;

    Ok(Json(ApiResponse::ok(apr_summary)))
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/apr/series",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("timeframe" = Timeframe, Query, description = "Time period for APR series", example = "7d")
    ),
    responses(
        (status = 200, description = "Historical APR data", body = AprSeriesDTO),
        (status = 400, description = "Invalid parameters"),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_apr_series(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
    Query(params): Query<AprSeriesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let timeframe = params.timeframe.as_str().to_owned();
    let apr_series = call_vault_backend(&client, &vault, "fetch APR series", move |backend| {
        let timeframe = timeframe.clone();
        async move { backend.get_vault_apr_series(&timeframe).await }
    })
    .await?;

    Ok(Json(ApiResponse::ok(apr_series)))
}
