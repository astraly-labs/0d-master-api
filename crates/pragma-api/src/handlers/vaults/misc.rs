use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    AppState,
    dto::{CapItem, VaultCapsResponse, VaultNavLatestResponse},
    errors::ApiError,
    helpers::VaultMasterAPIClient,
};
use pragma_db::models::Vault;

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/caps",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Current values vs configured limits", body = VaultCapsResponse),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_caps(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
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

    // Call the vault's caps endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let caps = client.get_vault_caps().await.map_err(|e| {
        tracing::error!("Failed to fetch vault caps: {}", e);
        ApiError::InternalServerError
    })?;

    // Convert the helper DTO to our API response DTO
    let response = VaultCapsResponse {
        items: caps
            .items
            .into_iter()
            .map(|item| CapItem {
                name: item.name,
                current: item.current,
                limit: item.limit,
                unit: item.unit,
            })
            .collect(),
    };

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/nav/latest",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Latest NAV report & deltas", body = VaultNavLatestResponse),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_nav_latest(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
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

    // Call the vault's NAV latest endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let nav_latest = client.get_vault_nav_latest().await.map_err(|e| {
        tracing::error!("Failed to fetch vault NAV latest: {}", e);
        ApiError::InternalServerError
    })?;

    // Convert the helper DTO to our API response DTO
    let response = VaultNavLatestResponse {
        date: nav_latest.date,
        aum: nav_latest.aum,
        var_since_prev_pct: nav_latest.var_since_prev_pct,
        apr_since_prev_pct: nav_latest.apr_since_prev_pct,
        report_url: nav_latest.report_url,
    };

    Ok(Json(response))
}
