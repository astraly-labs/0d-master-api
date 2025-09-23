use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use pragma_db::models::Vault;
use pragma_master::{CapsDTO, NavLatestDTO, VaultMasterAPIClient};

use crate::{AppState, dto::VaultInfoDTO, errors::ApiError};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/caps",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Current values vs configured limits", body = CapsDTO),
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

    Ok(Json(caps))
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/nav/latest",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Latest NAV report & deltas", body = NavLatestDTO),
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

    Ok(Json(nav_latest))
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/info",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Vault information including share price and AUM", body = VaultInfoDTO),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_info(
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

    // Call the vault's info endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let vault_info = client.get_vault_info().await.map_err(|e| {
        tracing::error!("Failed to fetch vault info: {}", e);
        ApiError::InternalServerError
    })?;

    // Convert the internal VaultInfoResponse to the public DTO
    let info_dto = VaultInfoDTO {
        current_epoch: vault_info.current_epoch,
        underlying_currency: vault_info.underlying_currency,
        pending_withdrawals_assets: vault_info.pending_withdrawals_assets,
        aum: vault_info.aum,
        buffer: vault_info.buffer,
        share_price_in_usd: vault_info.share_price_in_usd,
    };

    Ok(Json(info_dto))
}
