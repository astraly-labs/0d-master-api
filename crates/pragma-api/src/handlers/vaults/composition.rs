use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    AppState,
    dto::{
        VaultCompositionResponse, VaultCompositionSeriesResponse, 
        CompositionPosition, CompositionSeriesPoint,
    },
    errors::ApiError,
    helpers::VaultMasterAPIClient,
};
use pragma_db::models::Vault;

#[derive(Debug, Deserialize)]
pub struct CompositionQuery {
    #[serde(default = "default_group_by")]
    group_by: String,
}

fn default_group_by() -> String {
    "platform".to_string()
}

#[derive(Debug, Deserialize)]
pub struct CompositionSeriesQuery {
    #[serde(default = "default_timeframe")]
    timeframe: String,
    #[serde(default = "default_group_by")]
    group_by: String,
}

fn default_timeframe() -> String {
    "all".to_string()
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/composition",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("group_by" = String, Query, description = "Aggregate by platform or asset", example = "platform")
    ),
    responses(
        (status = 200, description = "Current vault composition", body = VaultCompositionResponse),
        (status = 400, description = "Invalid parameters"),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_composition(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
    Query(params): Query<CompositionQuery>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate group_by parameter
    if params.group_by != "platform" && params.group_by != "asset" {
        return Err(ApiError::BadRequest(
            "group_by must be either 'platform' or 'asset'".to_string(),
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

    // Call the vault's composition endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let composition = client
        .get_vault_composition(&params.group_by)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch vault composition: {}", e);
            ApiError::InternalServerError
        })?;

    // Convert the helper DTO to our API response DTO
    let response = VaultCompositionResponse {
        as_of: composition.as_of,
        positions: composition
            .positions
            .into_iter()
            .map(|p| CompositionPosition {
                platform: p.platform,
                asset: p.asset,
                symbol: p.symbol,
                pct: p.pct,
                apy_est_pct: p.apy_est_pct,
                icon: p.icon,
            })
            .collect(),
    };

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/composition/series",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("timeframe" = String, Query, description = "Time period for composition series", example = "30d"),
        ("group_by" = String, Query, description = "Group by platform or asset", example = "platform")
    ),
    responses(
        (status = 200, description = "Historical composition data", body = VaultCompositionSeriesResponse),
        (status = 400, description = "Invalid parameters"),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_composition_series(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
    Query(params): Query<CompositionSeriesQuery>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate timeframe parameter
    if !["7d", "30d", "1y", "all"].contains(&params.timeframe.as_str()) {
        return Err(ApiError::BadRequest(
            "timeframe must be one of: 7d, 30d, 1y, all".to_string(),
        ));
    }

    // Validate group_by parameter
    if params.group_by != "platform" && params.group_by != "asset" {
        return Err(ApiError::BadRequest(
            "group_by must be either 'platform' or 'asset'".to_string(),
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

    // Call the vault's composition series endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let composition_series = client
        .get_vault_composition_series(&params.timeframe, &params.group_by)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch vault composition series: {}", e);
            ApiError::InternalServerError
        })?;

    // Convert the helper DTO to our API response DTO
    let response = VaultCompositionSeriesResponse {
        timeframe: composition_series.timeframe,
        group_by: composition_series.group_by,
        labels: composition_series.labels,
        points: composition_series
            .points
            .into_iter()
            .map(|p| CompositionSeriesPoint {
                t: p.t,
                weights_pct: p.weights_pct,
            })
            .collect(),
    };

    Ok(Json(response))
}
