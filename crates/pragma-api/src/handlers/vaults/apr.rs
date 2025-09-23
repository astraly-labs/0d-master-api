use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    AppState,
    dto::{AprBasis, AprPoint, VaultAprSeriesResponse, VaultAprSummaryResponse},
    errors::ApiError,
    helpers::{AprSummaryBasis, VaultMasterAPIClient},
};
use pragma_db::models::Vault;

#[derive(Debug, Deserialize)]
pub struct AprSummaryQuery {
    #[serde(default = "default_apr_basis")]
    apr_basis: String,
}

fn default_apr_basis() -> String {
    "nominal".to_string()
}

#[derive(Debug, Deserialize)]
pub struct AprSeriesQuery {
    #[serde(default = "default_timeframe")]
    timeframe: String,
}

fn default_timeframe() -> String {
    "all".to_string()
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/apr/summary",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("apr_basis" = String, Query, description = "APR calculation basis", example = "nominal")
    ),
    responses(
        (status = 200, description = "APR summary", body = VaultAprSummaryResponse),
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
    // Validate apr_basis parameter
    if params.apr_basis != "nominal" && params.apr_basis != "inflation_adjusted" {
        return Err(ApiError::BadRequest(
            "apr_basis must be either 'nominal' or 'inflation_adjusted'".to_string(),
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

    // Call the vault's APR summary endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let apr_summary = client
        .get_vault_apr_summary(&params.apr_basis)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch vault APR summary: {}", e);
            ApiError::InternalServerError
        })?;

    // Convert the helper DTO to our API response DTO
    let response = VaultAprSummaryResponse {
        apr_pct: apr_summary.apr_pct,
        apr_basis: match apr_summary.apr_basis {
            AprSummaryBasis::Nominal => AprBasis::Nominal,
            AprSummaryBasis::InflationAdjusted => AprBasis::InflationAdjusted,
        },
    };

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/apr/series",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("timeframe" = String, Query, description = "Time period for APR series", example = "7d")
    ),
    responses(
        (status = 200, description = "Historical APR data", body = VaultAprSeriesResponse),
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
    // Validate timeframe parameter
    if !["7d", "30d", "1y", "all"].contains(&params.timeframe.as_str()) {
        return Err(ApiError::BadRequest(
            "timeframe must be one of: 7d, 30d, 1y, all".to_string(),
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

    // Call the vault's APR series endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let apr_series = client
        .get_vault_apr_series(&params.timeframe)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch vault APR series: {}", e);
            ApiError::InternalServerError
        })?;

    // Convert the helper DTO to our API response DTO
    let response = VaultAprSeriesResponse {
        timeframe: params.timeframe,
        points: apr_series
            .points
            .into_iter()
            .map(|p| AprPoint {
                t: p.t,
                apr_pct: p.apr_pct,
            })
            .collect(),
    };

    Ok(Json(response))
}
