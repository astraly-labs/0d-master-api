use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    AppState,
    dto::{ApiResponse, Vault as VaultDto},
    errors::ApiError,
    helpers::{call_vault_backend, fetch_vault_with_client, map_status},
};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Vault metadata with live values", body = VaultDto),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let (vault_stats, share_price) = call_vault_backend(
        &client,
        &vault,
        "fetch vault stats and share price",
        |backend| async move {
            let stats = backend.get_vault_stats().await?;
            let share_price = backend.get_vault_share_price().await?;
            Ok((stats, share_price))
        },
    )
    .await?;

    // Build response DTO and embed live values.
    let mut dto = VaultDto::from(vault);
    // Map status to spec: active -> live
    dto.status = map_status(&dto.status);
    dto.tvl = vault_stats.tvl;
    dto.share_price = share_price;

    Ok(Json(ApiResponse::ok(dto)))
}
