use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use pragma_db::models::Vault;
use pragma_master::{TimeseriesResponseDTO, VaultAlternativeAPIClient, VaultMasterAPIClient};

use crate::{
    AppState,
    dto::{ApiResponse, TimeseriesQuery},
    errors::ApiError,
    helpers::is_alternative_vault,
};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/timeseries",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("metric" = String, Query, description = "Vault-level metric to return", example = "tvl"),
        ("timeframe" = String, Query, description = "Time period", example = "7d"),
        ("currency" = String, Query, description = "Currency for values", example = "USD")
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
    // Validate metric parameter
    if params.metric != "tvl" && params.metric != "pnl" {
        return Err(ApiError::BadRequest(
            "metric must be either 'tvl' or 'pnl'".to_string(),
        ));
    }

    // Validate timeframe parameter
    if !["7d", "30d", "1y", "all"].contains(&params.timeframe.as_str()) {
        return Err(ApiError::BadRequest(
            "timeframe must be one of: 7d, 30d, 1y, all".to_string(),
        ));
    }

    // Validate currency parameter
    if params.currency != "USD" && params.currency != "USDC" {
        return Err(ApiError::BadRequest(
            "currency must be either 'USD' or 'USDC'".to_string(),
        ));
    }

    // Find the vault to get its API endpoint
    let conn = state.pool.get().await.map_err(|e| {
        tracing::error!("Failed to get database connection: {}", e);
        ApiError::InternalServerError
    })?;

    let vault_id_clone = vault_id.clone();
    let vault = conn
        .interact(move |conn| Vault::find_by_id(&vault_id_clone, conn))
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error: {}", e);
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            if e == diesel::result::Error::NotFound {
                ApiError::NotFound(format!("Vault {vault_id} not found"))
            } else {
                tracing::error!("Failed to fetch vault: {}", e);
                ApiError::InternalServerError
            }
        })?;

    // Call the vault's timeseries endpoint via helper
    let timeseries = if is_alternative_vault(&vault.id) {
        let client = VaultAlternativeAPIClient::new(&vault.api_endpoint, &vault.contract_address)
            .map_err(|e| {
            tracing::error!(
                "Failed to create alternative vault API client for vault {}: {}",
                vault_id,
                e
            );
            ApiError::InternalServerError
        })?;

        client
            .get_vault_timeseries(&params.metric, &params.timeframe, &params.currency)
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
        let client = VaultMasterAPIClient::new(&vault.api_endpoint).map_err(|e| {
            tracing::error!(
                "Failed to create vault API client for vault {}: {}",
                vault_id,
                e
            );
            ApiError::InternalServerError
        })?;

        client
            .get_vault_timeseries(&params.metric, &params.timeframe, &params.currency)
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
