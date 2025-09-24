use axum::{
    Json,
    extract::{Path, Query, State},
    response::IntoResponse,
};

use crate::{
    AppState,
    dto::{UserTransaction, UserTransactionHistory},
    errors::ApiError,
    helpers::validate_indexer_status,
};
use pragma_db::models::UserTransaction as DbUserTransaction;
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct TransactionQuery {
    #[serde(rename = "type")]
    pub transaction_type: Option<String>,
    pub limit: Option<i64>,
    pub cursor: Option<String>,
}

#[utoipa::path(
    get,
    path = "/users/{address}/vaults/{vault_id}/transactions",
    tag = "User",
    params(
        ("address" = String, Path, description = "User wallet address"),
        ("vault_id" = String, Path, description = "Vault identifier"),
        ("type" = Option<String>, Query, description = "Transaction type filter"),
        ("limit" = Option<i64>, Query, description = "Number of transactions to return (1-200, default: 50)"),
        ("cursor" = Option<String>, Query, description = "Pagination cursor")
    ),
    responses(
        (status = 200, description = "User transaction history", body = UserTransactionHistory),
        (status = 404, description = "User or vault not found"),
        (status = 503, description = "Indexer not synced or experiencing issues"),
        (status = 500, description = "Internal server error")
    )
)]
pub async fn get_user_transaction_history(
    State(state): State<AppState>,
    Path((address, vault_id)): Path<(String, String)>,
    Query(query): Query<TransactionQuery>,
) -> Result<impl IntoResponse, ApiError> {
    // Validate that the indexer is synced before serving user data
    validate_indexer_status(&vault_id, &state.pool).await?;

    let conn = state.pool.get().await.map_err(|e| {
        tracing::error!("Failed to get database connection: {}", e);
        ApiError::InternalServerError
    })?;

    // Validate and set limit
    let limit = query.limit.unwrap_or(50).clamp(1, 200);

    // Parse cursor for pagination (using transaction ID)
    let cursor_id: Option<i32> = query.cursor.and_then(|c| c.parse().ok());

    let address_clone = address.clone();
    let vault_id_clone = vault_id.clone();
    let transaction_type_clone = query.transaction_type.clone();

    let transactions = conn
        .interact(move |conn| {
            DbUserTransaction::find_by_user_and_vault_paginated(
                &address_clone,
                &vault_id_clone,
                transaction_type_clone.as_deref(),
                cursor_id,
                limit + 1, // Get one extra to determine if there's a next page
                conn,
            )
        })
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error: {}", e);
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            tracing::error!("Failed to fetch user transactions: {}", e);
            ApiError::InternalServerError
        })?;

    // Determine if there are more results and set next cursor
    let has_more = transactions.len() > limit as usize;
    let transactions_to_return = if has_more {
        &transactions[..limit as usize]
    } else {
        &transactions
    };

    let next_cursor = if has_more {
        transactions_to_return.last().map(|tx| tx.id.to_string())
    } else {
        None
    };

    // Convert to DTOs
    let items: Vec<UserTransaction> = transactions_to_return
        .iter()
        .map(|tx| UserTransaction::from(tx.clone()))
        .collect();

    let response = UserTransactionHistory { items, next_cursor };

    Ok(Json(response))
}
