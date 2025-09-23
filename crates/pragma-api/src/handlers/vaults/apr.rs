use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use pragma_db::models::Vault;
use pragma_master::{AprSeriesDTO, AprSummaryDTO, VaultMasterAPIClient};

use crate::{
    AppState,
    dto::{AprSeriesQuery, AprSummaryQuery},
    errors::ApiError,
};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/apr/summary",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("apr_basis" = String, Query, description = "APR calculation basis", example = "nominal")
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

    Ok(Json(apr_summary))
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

    Ok(Json(apr_series))
}
