use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use pragma_db::models::Vault;
use pragma_master::{KpisDTO, VaultMasterAPIClient};

use crate::{AppState, dto::TimeframeQuery, errors::ApiError};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/kpis",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("timeframe" = String, Query, description = "Time period for KPI calculation", example = "all")
    ),
    responses(
        (status = 200, description = "Vault performance KPIs", body = KpisDTO),
        (status = 400, description = "Invalid parameters"),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_kpis(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
    Query(params): Query<TimeframeQuery>,
) -> Result<impl IntoResponse, ApiError> {
    tracing::info!("Getting vault KPIs for {:?}", params);

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

    // Call the vault's KPIs endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let kpis = client
        .get_vault_kpis(&params.timeframe)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch vault KPIs for vault {}: {}", vault_id, e);
            ApiError::InternalServerError
        })?;

    Ok(Json(kpis))
}
