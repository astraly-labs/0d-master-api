use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use zerod_db::{
    ZerodPool,
    models::Vault,
    types::{Currency, Metric, Timeframe},
};
use zerod_master::{JaffarClient, TimeseriesResponseDTO, VaultMasterClient, VesuClient};

use crate::{
    AppState,
    dto::{ApiResponse, TimeseriesQuery},
    errors::{ApiError, DatabaseErrorExt},
    helpers::is_alternative_vault,
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
    let vault_id_clone = vault_id.clone();
    let vault = state
        .pool
        .interact_with_context(format!("find vault by id: {vault_id}"), move |conn| {
            Vault::find_by_id(&vault_id_clone, conn)
        })
        .await
        .map_err(|e| e.or_not_found(format!("Vault {vault_id} not found")))?;

    // Call the vault's timeseries endpoint via helper
    let timeseries = if is_alternative_vault(&vault.id) {
        let client =
            VesuClient::new(&vault.api_endpoint, &vault.contract_address).map_err(|e| {
                tracing::error!(
                    "Failed to create alternative vault API client for vault {}: {}",
                    vault_id,
                    e
                );
                ApiError::InternalServerError
            })?;

        client
            .get_vault_timeseries(
                params.metric.as_str(),
                params.timeframe.as_str(),
                params.currency.as_str(),
            )
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to fetch alternative vault timeseries for vault {}: {}",
                    vault_id,
                    e
                );
                ApiError::InternalServerError
            })?
    } else {
        let client = JaffarClient::new(&vault.api_endpoint);

        client
            .get_vault_timeseries(
                params.metric.as_str(),
                params.timeframe.as_str(),
                params.currency.as_str(),
            )
            .await
            .map_err(|e| {
                tracing::error!(
                    "Failed to fetch vault timeseries for vault {}: {}",
                    vault_id,
                    e
                );
                ApiError::InternalServerError
            })?
    };

    Ok(Json(ApiResponse::ok(timeseries)))
}
