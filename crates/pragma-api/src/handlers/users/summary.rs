use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};

use crate::{
    AppState,
    dto::{ApiResponse, UserPositionSummary},
    errors::ApiError,
    helpers::validate_indexer_status,
};
use chrono::Utc;
use pragma_db::models::{UserPosition, UserTransaction, Vault};
use pragma_master::VaultMasterAPIClient;
use rust_decimal::Decimal;

#[utoipa::path(
    get,
    path = "/users/{address}/vaults/{vault_id}/summary",
    tag = "User",
    params(
        ("address" = String, Path, description = "User wallet address"),
        ("vault_id" = String, Path, description = "Vault identifier")
    ),
    responses(
        (status = 200, description = "User position summary", body = UserPositionSummary),
        (status = 404, description = "User position not found"),
        (status = 503, description = "Indexer not synced or experiencing issues"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_user_position_summary(
    State(state): State<AppState>,
    Path((address, vault_id)): Path<(String, String)>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate that the indexer is synced before serving user data
    validate_indexer_status(&vault_id, &state.pool).await?;

    let conn = state.pool.get().await.map_err(|e| {
        tracing::error!("Failed to get database connection: {}", e);
        ApiError::InternalServerError
    })?;

    // Find the user's position in the vault
    let address_clone = address.clone();
    let vault_id_clone = vault_id.clone();
    let position = conn
        .interact(move |conn| {
            UserPosition::find_by_user_and_vault(&address_clone, &vault_id_clone, conn)
        })
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error: {}", e);
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            if e == diesel::result::Error::NotFound {
                ApiError::NotFound(format!(
                    "User {address} position in vault {vault_id} not found"
                ))
            } else {
                tracing::error!("Failed to fetch user position: {}", e);
                ApiError::InternalServerError
            }
        })?;

    // Get vault metadata to fetch share price
    let vault_id_for_vault = vault_id.clone();
    let vault = conn
        .interact(move |conn| Vault::find_by_id(&vault_id_for_vault, conn))
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

    // Fetch current share price from vault API
    let client = VaultMasterAPIClient::new(&vault.api_endpoint)?;
    let share_price_str = client.get_vault_share_price().await.map_err(|e| {
        tracing::error!("Failed to fetch vault share price: {}", e);
        ApiError::InternalServerError
    })?;

    let share_price = share_price_str.parse::<Decimal>().map_err(|e| {
        tracing::error!("Failed to parse share price '{share_price_str}': {e}");
        ApiError::InternalServerError
    })?;

    // Calculate total deposits using database query
    let address_for_deposits = address.clone();
    let vault_id_for_deposits = vault_id.clone();
    let total_deposits = conn
        .interact(move |conn| {
            UserTransaction::total_deposits_by_user_and_vault(
                &address_for_deposits,
                &vault_id_for_deposits,
                conn,
            )
            .map(std::option::Option::unwrap_or_default)
        })
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error: {}", e);
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            tracing::error!("Failed to calculate total deposits: {}", e);
            ApiError::InternalServerError
        })?;

    // Calculate position metrics
    let position_value = position.share_balance * share_price;
    let all_time_earned = position_value.saturating_sub(total_deposits);

    let summary = UserPositionSummary {
        vault_id: vault_id.clone(),
        as_of: Utc::now(),
        position_value_usd: position_value.to_string(),
        share_balance: position.share_balance.to_string(),
        share_price: share_price.to_string(),
        first_deposit_at: position.first_deposit_at,
        total_deposits: total_deposits.to_string(),
        all_time_earned: all_time_earned.to_string(),
    };

    Ok(Json(ApiResponse::ok(summary)))
}
