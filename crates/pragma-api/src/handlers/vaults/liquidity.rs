use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    AppState,
    dto::{
        InstantLiquidity, LiquiditySimulateRequest, LiquiditySimulateResponse, ScheduledWindow,
        SlippagePoint, VaultLiquidityResponse, VaultSlippageCurveResponse,
    },
    errors::ApiError,
    helpers::VaultMasterAPIClient,
};
use pragma_db::models::Vault;

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/liquidity",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Vault liquidity summary", body = VaultLiquidityResponse),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_liquidity(
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

    // Call the vault's liquidity endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let liquidity = client.get_vault_liquidity().await.map_err(|e| {
        tracing::error!("Failed to fetch vault liquidity: {}", e);
        ApiError::InternalServerError
    })?;

    // Convert the helper DTO to our API response DTO
    let response = VaultLiquidityResponse {
        as_of: liquidity.as_of,
        is_liquid: liquidity.is_liquid,
        withdraw_capacity_usd_24h: liquidity.withdraw_capacity_usd_24h,
        deposit_capacity_usd_24h: liquidity.deposit_capacity_usd_24h,
        policy_markdown: liquidity.policy_markdown,
    };

    Ok(Json(response))
}

#[utoipa::path(
    get,
    path = "/vaults/{vault_id}/liquidity/curve",
    tag = "Vaults",
    params(
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "Vault slippage curve", body = VaultSlippageCurveResponse),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_vault_slippage_curve(
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

    // Call the vault's slippage curve endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let curve = client.get_vault_slippage_curve().await.map_err(|e| {
        tracing::error!("Failed to fetch vault slippage curve: {}", e);
        ApiError::InternalServerError
    })?;

    // Convert the helper DTO to our API response DTO
    let response = VaultSlippageCurveResponse {
        is_liquid: curve.is_liquid,
        points: curve
            .points
            .into_iter()
            .map(|p| SlippagePoint {
                amount_usd: p.amount_usd,
                slippage_bps: p.slippage_bps,
            })
            .collect(),
    };

    Ok(Json(response))
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
        (status = 200, description = "Liquidity simulation result", body = LiquiditySimulateResponse),
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

    // Call the vault's liquidity simulation endpoint via helper
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let simulation = client
        .simulate_liquidity(&request.amount)
        .await
        .map_err(|e| {
            tracing::error!("Failed to simulate vault liquidity: {}", e);
            ApiError::InternalServerError
        })?;

    // Convert the helper DTO to our API response DTO
    let response = LiquiditySimulateResponse {
        amount: simulation.amount,
        instant: simulation.instant.map(|instant| InstantLiquidity {
            supported: instant.supported,
            est_slippage_bps: instant.est_slippage_bps,
            cap_remaining: instant.cap_remaining,
        }),
        scheduled: simulation
            .scheduled
            .into_iter()
            .map(|window| ScheduledWindow {
                window: window.window,
                max_without_delay: window.max_without_delay,
                expected_nav_date: window.expected_nav_date,
            })
            .collect(),
    };

    Ok(Json(response))
}
