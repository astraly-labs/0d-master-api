use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use pragma_db::models::Vault;
use pragma_master::{CompositionDTO, CompositionSeriesDTO, VaultMasterAPIClient};

use crate::{
    AppState,
    dto::{ApiResponse, CompositionQuery, CompositionSeriesQuery},
    errors::ApiError,
};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/composition",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("group_by" = String, Query, description = "Aggregate by platform or asset", example = "platform")
    ),
    responses(
        (status = 200, description = "Current vault composition", body = CompositionDTO),
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

    Ok(Json(ApiResponse::ok(composition)))
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
        (status = 200, description = "Historical composition data", body = CompositionSeriesDTO),
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

    Ok(Json(ApiResponse::ok(composition_series)))
}
