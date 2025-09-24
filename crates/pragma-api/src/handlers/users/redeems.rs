use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};
use chrono::Utc;
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::{
    AppState,
    dto::{ApiResponse, PendingRedeem, user::PendingRedeemsResponse},
    errors::ApiError,
};
use pragma_db::models::{TransactionStatus, User, user_transaction::UserTransaction};

#[derive(Debug, Deserialize)]
pub struct PendingRedeemsQuery {
    pub vault_id: Option<String>,
    pub asset_type: Option<String>,
}

#[utoipa::path(
    get,
    path = "/users/{address}/redeems",
    tag = "User",
    params(
        ("address" = String, Path, description = "User wallet address"),
        ("vault_id" = Option<String>, Query, description = "Filter by vault ID"),
        ("asset_type" = Option<String>, Query, description = "Filter by asset type")
    ),
    responses(
        (status = 200, description = "Pending redeems for user", body = PendingRedeemsResponse),
        (status = 404, description = "User not found"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_user_pending_redeems(
    State(state): State<AppState>,
    Path(address): Path<String>,
    Query(query): Query<PendingRedeemsQuery>,
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
    let pending_user_txs: Vec<UserTransaction> = conn
        .interact(move |conn| {
            use pragma_db::schema::user_transactions::dsl;

            let mut query = dsl::user_transactions
                .filter(dsl::user_address.eq(&address_clone))
                .filter(dsl::status.eq(TransactionStatus::Pending.as_str()))
                .into_boxed();

            if let Some(vault_id) = vault_id_filter {
                query = query.filter(dsl::vault_id.eq(vault_id));
            }

            query
                .order(dsl::block_timestamp.desc())
                .load::<UserTransaction>(conn)
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
    let pending_redeems: Vec<PendingRedeem> = pending_user_txs
        .into_iter()
        .map(PendingRedeem::from)
        .collect();

    // Calculate total pending USD
    let total_pending: Decimal = pending_redeems
        .iter()
        .map(|asset| asset.amount.parse::<Decimal>().unwrap_or_default())
        .sum();

    let response = PendingRedeemsResponse {
        address: address.clone(),
        as_of: Utc::now(),
        pending_redeems,
        total_pending: total_pending.to_string(),
    };

    Ok(Json(ApiResponse::ok(response)))
}
