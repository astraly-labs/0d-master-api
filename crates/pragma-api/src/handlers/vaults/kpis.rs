use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{AppState, dto::VaultKpisResponse, errors::ApiError, helpers::VaultMasterAPIClient};
use pragma_db::models::Vault;

#[derive(Debug, Deserialize)]
pub struct KpisQuery {
    #[serde(default = "default_timeframe")]
    timeframe: String,
}

fn default_timeframe() -> String {
    "all".to_string()
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/kpis",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("timeframe" = String, Query, description = "Time period for KPI calculation", example = "all")
    ),
    responses(
        (status = 200, description = "Vault performance KPIs", body = VaultKpisResponse),
        (status = 400, description = "Invalid parameters"),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_kpis(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
    Query(params): Query<KpisQuery>,
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

    // Call the vault's KPIs endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let kpis = client
        .get_vault_kpis(&params.timeframe)
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch vault KPIs for vault {}: {}", vault_id, e);
            ApiError::InternalServerError
        })?;

    // Convert the helper DTO to our API response DTO
    let response = VaultKpisResponse {
        cumulative_pnl_usd: kpis.cumulative_pnl_usd,
        max_drawdown_pct: kpis.max_drawdown_pct,
        sharpe: kpis.sharpe,
        profit_share_bps: kpis.profit_share_bps,
    };

    Ok(Json(response))
}
