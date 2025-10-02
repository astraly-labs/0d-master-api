use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use zerod_db::{ZerodPool, models::Vault, types::Timeframe};
use zerod_master::{KpisDTO, VaultMasterAPIClient};

use crate::{
    AppState,
    dto::{ApiResponse, TimeframeQuery},
    errors::{ApiError, DatabaseErrorExt},
};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/kpis",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("timeframe" = Timeframe, Query, description = "Time period for KPI calculation", example = "all")
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

    let vault_id_clone = vault_id.clone();
    let vault = state
        .pool
        .interact_with_context(format!("find vault by id: {vault_id}"), move |conn| {
            Vault::find_by_id(&vault_id_clone, conn)
        })
        .await
        .map_err(|e| e.or_not_found(format!("Vault {vault_id} not found")))?;

    // Call the vault's KPIs endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let kpis = client
        .get_vault_kpis(params.timeframe.as_str())
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch vault KPIs for vault {}: {}", vault_id, e);
            ApiError::InternalServerError
        })?;

    Ok(Json(ApiResponse::ok(kpis)))
}
