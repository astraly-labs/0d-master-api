use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use chrono::Utc;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::{
    AppState,
    dto::{PendingAsset, user::PendingAssetsResponse},
    errors::ApiError,
};
use pragma_db::models::{TransactionStatus, User, user_transaction::UserTransaction};

#[derive(Debug, Deserialize)]
pub struct PendingAssetsQuery {
    pub vault_id: Option<String>,
    pub asset_type: Option<String>,
}

#[utoipa::path(
    get,
    path = "/users/{address}/pending-assets",
    tag = "User",
    params(
        ("address" = String, Path, description = "User wallet address"),
        ("vault_id" = Option<String>, Query, description = "Filter by vault ID"),
        ("asset_type" = Option<String>, Query, description = "Filter by asset type")
    ),
    responses(
        (status = 200, description = "Pending assets for user", body = PendingAssetsResponse),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_user_pending_assets(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(query): Query<PendingAssetsQuery>,
) -> Result<impl IntoResponse, ApiError> {
    let conn = state.pool.get().await.map_err(|e| {
        tracing::error!("Failed to get database connection: {e}");
        ApiError::InternalServerError
    })?;
    // First verify the user exists
    let address_clone = address.clone();
    let _ = conn
        .interact(move |conn| User::find_by_address(&address_clone, conn))
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error: {e}");
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            if e == diesel::result::Error::NotFound {
                ApiError::NotFound(format!("User {address} not found"))
            } else {
                tracing::error!("Failed to fetch user: {e}");
                ApiError::InternalServerError
            }
        })?;

    // Get pending transactions for the user
    let address_clone = address.clone();
    let vault_id_filter = query.vault_id.clone();
    let pending_transactions: Vec<UserTransaction> = conn
        .interact(move |conn| {
            let mut query_builder = UserTransaction::find_by_user(&address_clone, conn)?;

            // Filter by vault_id if provided
            if let Some(vault_id) = vault_id_filter {
                query_builder.retain(|tx| tx.vault_id == vault_id);
            }

            // Filter by pending status
            let pending_txs: Vec<UserTransaction> = query_builder
                .into_iter()
                .filter(|tx| tx.status == TransactionStatus::Pending.as_str())
                .collect();

            Ok(pending_txs)
        })
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error: {e}");
            ApiError::InternalServerError
        })?
        .map_err(|e: diesel::result::Error| {
            tracing::error!("Failed to fetch pending transactions: {e}");
            ApiError::InternalServerError
        })?;

    // Convert to PendingAsset DTOs
    let pending_assets: Vec<_> = pending_transactions
        .into_iter()
        .map(PendingAsset::from)
        .collect();

    // Calculate total pending USD
    let total_pending: Decimal = pending_assets
        .iter()
        .map(|asset| asset.amount.parse::<Decimal>().unwrap_or_default())
        .sum();

    let response = PendingAssetsResponse {
        address: address.clone(),
        as_of: Utc::now(),
        pending_assets,
        total_pending: total_pending.to_string(),
    };

    Ok(Json(response))
}
