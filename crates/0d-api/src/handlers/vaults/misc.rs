use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use zerod_master::{CapsDTO, NavLatestDTO};

use crate::{
    AppState,
    dto::{ApiResponse, VaultInfoDTO},
    errors::ApiError,
    helpers::{call_vault_backend, fetch_vault_with_client},
};

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
    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let caps = call_vault_backend(&client, &vault, "fetch vault caps", |backend| async move {
        backend.get_vault_caps().await
    })
    .await?;

    Ok(Json(ApiResponse::ok(caps)))
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
    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let nav_latest = call_vault_backend(
        &client,
        &vault,
        "fetch vault nav latest",
        |backend| async move { backend.get_vault_nav_latest().await },
    )
    .await?;

    Ok(Json(ApiResponse::ok(nav_latest)))
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
    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let vault_info =
        call_vault_backend(&client, &vault, "fetch vault info", |backend| async move {
            backend.get_vault_info().await
        })
        .await?;

    // Convert the internal VaultInfoResponse to the public DTO
    let info_dto = VaultInfoDTO {
        current_epoch: vault_info.current_epoch,
        underlying_currency: vault_info.underlying_currency,
        underlying_currency_address: vault_info.underlying_currency_address,
        pending_withdrawals_assets: vault_info.pending_withdrawals_assets,
        aum: vault_info.aum,
        buffer: vault_info.buffer,
        share_price_in_usd: vault_info.share_price_in_usd,
    };

    Ok(Json(ApiResponse::ok(info_dto)))
}
