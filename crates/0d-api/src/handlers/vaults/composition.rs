use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use zerod_db::{
    ZerodPool,
    models::Vault,
    types::{GroupBy, Timeframe},
};
use zerod_master::{CompositionDTO, CompositionSeriesDTO, JaffarClient, VaultMasterClient};

use crate::{
    AppState,
    dto::{ApiResponse, CompositionQuery, CompositionSeriesQuery},
    errors::{ApiError, DatabaseErrorExt},
    helpers::{call_vault_backend, fetch_vault_with_client},
};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/composition",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("group_by" = GroupBy, Query, description = "Aggregate by platform or asset", example = "platform")
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
    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let group_by = params.group_by.as_str().to_owned();
    let composition = call_vault_backend(
        &client,
        &vault,
        "fetch vault composition",
        move |backend| {
            let group_by = group_by.clone();
            async move { backend.get_vault_composition(&group_by).await }
        },
    )
    .await?;

    Ok(Json(ApiResponse::ok(composition)))
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/composition/series",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("timeframe" = Timeframe, Query, description = "Time period for composition series", example = "30d"),
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
    let vault_id_clone = vault_id.clone();
    let vault = state
        .pool
        .interact_with_context(format!("find vault by id: {vault_id}"), move |conn| {
            Vault::find_by_id(&vault_id_clone, conn)
        })
        .await
        .map_err(|e| e.or_not_found(format!("Vault {vault_id} not found")))?;

    // Call the vault's composition series endpoint via helper
    let client = JaffarClient::new(&vault.api_endpoint);
    let composition_series = client
        .get_vault_composition_series(params.timeframe.as_str(), params.group_by.as_str())
        .await
        .map_err(|e| {
            tracing::error!("Failed to fetch vault composition series: {}", e);
            ApiError::InternalServerError
        })?;

    Ok(Json(ApiResponse::ok(composition_series)))
}
