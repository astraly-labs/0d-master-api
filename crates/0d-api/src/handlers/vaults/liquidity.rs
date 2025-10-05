use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use zerod_master::{LiquidityDTO, LiquiditySimulateResponseDTO, SlippageCurveDTO};

use crate::{
    AppState,
    dto::{ApiResponse, LiquiditySimulateRequest},
    errors::ApiError,
    helpers::{call_vault_backend, fetch_vault_with_client},
};

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/liquidity",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Vault liquidity summary", body = LiquidityDTO),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_liquidity(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let liquidity = call_vault_backend(
        &client,
        &vault,
        "fetch vault liquidity",
        |backend| async move { backend.get_vault_liquidity().await },
    )
    .await?;

    Ok(Json(ApiResponse::ok(liquidity)))
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/liquidity/curve",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Vault slippage curve", body = SlippageCurveDTO),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_slippage_curve(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
) -> Result<impl IntoResponse, ApiError> {
    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let curve = call_vault_backend(
        &client,
        &vault,
        "fetch vault slippage curve",
        |backend| async move { backend.get_vault_slippage_curve().await },
    )
    .await?;

    Ok(Json(ApiResponse::ok(curve)))
}

#[utoipa::path(
    post,
    path = "/vaults/{vault_id}/liquidity/simulate",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    request_body = LiquiditySimulateRequest,
    responses(
        (status = 200, description = "Liquidity simulation result", body = LiquiditySimulateResponseDTO),
        (status = 400, description = "Invalid request body"),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn simulate_vault_liquidity(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
    Json(request): Json<LiquiditySimulateRequest>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate amount is a valid number string
    if request.amount.parse::<f64>().is_err() {
        return Err(ApiError::BadRequest(
            "amount must be a valid number".to_string(),
        ));
    }

    let (vault, client) = fetch_vault_with_client(&state, &vault_id).await?;
    let amount = request.amount.clone();
    let simulation = call_vault_backend(
        &client,
        &vault,
        "simulate vault liquidity",
        move |backend| async move { backend.simulate_liquidity(&amount).await },
    )
    .await?;

    Ok(Json(ApiResponse::ok(simulation)))
}
