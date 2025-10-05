use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use zerod_db::{
    ZerodPool,
    models::Vault,
    types::{AprBasis, Timeframe},
};
use zerod_master::{AprSeriesDTO, AprSummaryDTO, JaffarClient, VaultMasterClient, VesuClient};

use crate::{
    AppState,
    dto::{ApiResponse, AprSeriesQuery, AprSummaryQuery},
    errors::{ApiError, DatabaseErrorExt},
    helpers::is_alternative_vault,
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
    let vault_id_clone = vault_id.clone();
    let vault = state
        .pool
        .interact_with_context(format!("find vault by id: {vault_id}"), move |conn| {
            Vault::find_by_id(&vault_id_clone, conn)
        })
        .await
        .map_err(|e| e.or_not_found(format!("Vault {vault_id} not found")))?;

    // Call the vault's APR summary endpoint via helper
    let apr_summary = if is_alternative_vault(&vault.id) {
        let client = VesuClient::new(&vault.api_endpoint, &vault.contract_address)?;
        client
            .get_vault_apr_summary(params.apr_basis.as_str())
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch alternative vault APR summary: {}", e);
                ApiError::InternalServerError
            })?
    } else {
        let client = JaffarClient::new(&vault.api_endpoint);
        client
            .get_vault_apr_summary(params.apr_basis.as_str())
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch vault APR summary: {}", e);
                ApiError::InternalServerError
            })?
    };

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
    let vault_id_clone = vault_id.clone();
    let vault = state
        .pool
        .interact_with_context(format!("find vault by id: {vault_id}"), move |conn| {
            Vault::find_by_id(&vault_id_clone, conn)
        })
        .await
        .map_err(|e| e.or_not_found(format!("Vault {vault_id} not found")))?;

    // Call the vault's APR series endpoint via helper
    let apr_series = if is_alternative_vault(&vault.id) {
        let client = VesuClient::new(&vault.api_endpoint, &vault.contract_address)?;
        client
            .get_vault_apr_series(params.timeframe.as_str())
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch alternative vault APR series: {e}");
                ApiError::InternalServerError
            })?
    } else {
        let client = JaffarClient::new(&vault.api_endpoint);
        client
            .get_vault_apr_series(params.timeframe.as_str())
            .await
            .map_err(|e| {
                tracing::error!("Failed to fetch vault APR series: {e}");
                ApiError::InternalServerError
            })?
    };

    Ok(Json(ApiResponse::ok(apr_series)))
}
