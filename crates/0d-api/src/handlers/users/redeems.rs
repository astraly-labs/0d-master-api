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
    dto::{ApiResponse, PendingRedeem, user::PendingRedeemsResponse},
    errors::{ApiError, DatabaseErrorExt},
};
use zerod_db::{
    ZerodPool,
    models::{User, user_transaction::UserTransaction},
};

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
    // First verify the user exists
    let address_clone = address.clone();
    let _ = state
        .pool
        .interact_with_context(format!("find user by address: {address}"), move |conn| {
            User::find_by_address(&address_clone, conn)
        })
        .await
        .map_err(|e| e.or_not_found(format!("User {address} not found")))?;

    // Get pending transactions for the user
    let address_clone = address.clone();
    let vault_id_filter = query.vault_id.clone();
    let pending_user_txs: Vec<UserTransaction> = state
        .pool
        .interact_with_context(
            format!("fetch pending transactions for user: {address}"),
            move |conn| {
                UserTransaction::find_pending_by_user(
                    &address_clone,
                    vault_id_filter.as_deref(),
                    conn,
                )
            },
        )
        .await?;

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

    // Calculate average redeem delay
    let address_clone = address.clone();
    let vault_id_for_delay = query.vault_id.clone();
    let average_redeem_delay = state
        .pool
        .interact_with_context(
            format!("calculate average redeem delay for user: {address}"),
            move |conn| {
                UserTransaction::calculate_average_redeem_delay(
                    &address_clone,
                    vault_id_for_delay.as_deref(),
                    conn,
                )
            },
        )
        .await?;

    let response = PendingRedeemsResponse {
        address: address.clone(),
        as_of: Utc::now(),
        pending_redeems,
        total_pending: total_pending.to_string(),
        average_redeem_delay,
    };

    Ok(Json(ApiResponse::ok(response)))
}
