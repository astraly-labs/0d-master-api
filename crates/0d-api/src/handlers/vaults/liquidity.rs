use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use zerod_db::{ZerodPool, models::Vault};
use zerod_master::{
    JaffarClient, LiquidityDTO, LiquiditySimulateResponseDTO, SlippageCurveDTO, VaultMasterClient,
};

use crate::{
    AppState,
    dto::{ApiResponse, LiquiditySimulateRequest},
    errors::{ApiError, DatabaseErrorExt},
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
    let vault_id_clone = vault_id.clone();
    let vault = state
        .pool
        .interact_with_context(format!("find vault by id: {vault_id}"), move |conn| {
            Vault::find_by_id(&vault_id_clone, conn)
        })
        .await
        .map_err(|e| e.or_not_found(format!("Vault {vault_id} not found")))?;

    // Call the vault's liquidity endpoint via helper
    let client = JaffarClient::new(&vault.api_endpoint);
    let liquidity = client.get_vault_liquidity().await.map_err(|e| {
        tracing::error!("Failed to fetch vault liquidity: {}", e);
        ApiError::InternalServerError
    })?;

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
    let vault_id_clone = vault_id.clone();
    let vault = state
        .pool
        .interact_with_context(format!("find vault by id: {vault_id}"), move |conn| {
            Vault::find_by_id(&vault_id_clone, conn)
        })
        .await
        .map_err(|e| e.or_not_found(format!("Vault {vault_id} not found")))?;

    // Call the vault's slippage curve endpoint via helper
    let client = JaffarClient::new(&vault.api_endpoint);
    let curve = client.get_vault_slippage_curve().await.map_err(|e| {
        tracing::error!("Failed to fetch vault slippage curve: {}", e);
        ApiError::InternalServerError
    })?;

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

    let vault_id_clone = vault_id.clone();
    let vault = state
        .pool
        .interact_with_context(format!("find vault by id: {vault_id}"), move |conn| {
            Vault::find_by_id(&vault_id_clone, conn)
        })
        .await
        .map_err(|e| e.or_not_found(format!("Vault {vault_id} not found")))?;

    // Call the vault's liquidity simulation endpoint via helper
    let client = JaffarClient::new(&vault.api_endpoint);
    let simulation = client
        .simulate_liquidity(&request.amount)
        .await
        .map_err(|e| {
            tracing::error!("Failed to simulate vault liquidity: {}", e);
            ApiError::InternalServerError
        })?;

    Ok(Json(ApiResponse::ok(simulation)))
}
