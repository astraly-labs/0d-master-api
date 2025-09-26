use std::str::FromStr;

use axum::{
    Json,
    extract::{Path, State},
    response::IntoResponse,
};
use bigdecimal::BigDecimal;
use chrono::Utc;
use serde_json::Value;
use starknet::{
    core::types::{BroadcastedInvokeTransaction, Felt},
    providers::Provider,
};
use uuid::Uuid;

use crate::{
    AppState,
    dto::{ApiResponse, VaultDepositRequest, VaultDepositResponse, VaultDepositStatus},
    errors::ApiError,
};
use pragma_db::models::{
    DepositRequest, DepositRequestStatus, DepositRequestUpdate, NewDepositRequest, Vault,
};

#[utoipa::path(
    post,
    path = "/vaults/{vault_id}/deposit",
    tag = "Vaults",
    params(("vault_id" = String, Path, description = "Vault identifier")),
    request_body = VaultDepositRequest,
    responses(
        (status = 200, description = "Deposit request accepted", body = VaultDepositResponse),
        (status = 400, description = "Invalid deposit request"),
        (status = 404, description = "Vault not found"),
        (status = 503, description = "Failed to submit transaction"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn submit_vault_deposit(
    State(state): State<AppState>,
    Path(vault_id): Path<String>,
    Json(payload): Json<VaultDepositRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let conn = state.pool.get().await.map_err(|e| {
        tracing::error!("Failed to get database connection: {e}");
        ApiError::InternalServerError
    })?;

    let vault = {
        let vault_id_clone = vault_id.clone();
        conn.interact(move |conn| Vault::find_by_id(&vault_id_clone, conn))
            .await
            .map_err(|e| {
                tracing::error!("Database interaction error: {e}");
                ApiError::InternalServerError
            })?
            .map_err(|e| match e {
                diesel::result::Error::NotFound => {
                    ApiError::NotFound(format!("Vault {vault_id} not found"))
                }
                _ => {
                    tracing::error!("Failed to fetch vault {vault_id}: {e}");
                    ApiError::InternalServerError
                }
            })?
    };

    if vault.chain.to_ascii_lowercase().ne("starknet") {
        return Err(ApiError::BadRequest(
            "Deposits are only supported for Starknet vaults".to_string(),
        ));
    }

    if vault.deposit_paused.unwrap_or(false) {
        return Err(ApiError::BadRequest(
            "Deposits are currently paused for this vault".to_string(),
        ));
    }

    let amount = BigDecimal::from_str(&payload.amount)
        .map_err(|_| ApiError::BadRequest("Invalid amount format".to_string()))?;
    if amount <= BigDecimal::from(0) {
        return Err(ApiError::BadRequest(
            "Deposit amount must be greater than zero".to_string(),
        ));
    }

    if let Some(ref min) = vault.min_deposit {
        if amount < *min {
            return Err(ApiError::BadRequest(format!(
                "Deposit amount below minimum threshold ({min})"
            )));
        }
    }

    if let Some(ref max) = vault.max_deposit {
        if amount > *max {
            return Err(ApiError::BadRequest(format!(
                "Deposit amount exceeds maximum threshold ({max})"
            )));
        }
    }

    let transaction_value: Value = payload.transaction.clone();
    let invoke_transaction: BroadcastedInvokeTransaction =
        serde_json::from_value(transaction_value.clone()).map_err(|e| {
            tracing::warn!("Invalid Starknet transaction payload: {e}");
            ApiError::BadRequest("Invalid transaction payload".to_string())
        })?;

    let provided_user_address = Felt::from_hex(&payload.user_address)
        .map_err(|_| ApiError::BadRequest("Invalid user address".to_string()))?;
    let transaction_sender = invoke_transaction.sender_address;

    if provided_user_address != transaction_sender {
        return Err(ApiError::BadRequest(
            "Transaction sender does not match user address".to_string(),
        ));
    }

    let request_id = Uuid::new_v4().to_string();
    let normalized_user_address = payload.user_address.to_ascii_lowercase();
    let referral_code = payload.referral_code.clone();

    let new_request = NewDepositRequest {
        id: request_id.clone(),
        vault_id: vault_id.clone(),
        user_address: normalized_user_address.clone(),
        amount: amount.clone(),
        referral_code: referral_code.clone(),
        transaction: transaction_value,
        tx_hash: None,
        status: DepositRequestStatus::Pending.as_str().to_string(),
        error_code: None,
        error_message: None,
    };

    conn.interact({
        let new_request = new_request.clone();
        move |conn| DepositRequest::create(&new_request, conn)
    })
    .await
    .map_err(|e| {
        tracing::error!("Failed to create deposit request: {e}");
        ApiError::InternalServerError
    })?
    .map_err(|e| {
        tracing::error!("Database error while creating deposit request: {e}");
        ApiError::InternalServerError
    })?;

    let submission_result = state
        .starknet_provider
        .add_invoke_transaction(&invoke_transaction)
        .await;

    match submission_result {
        Ok(result) => {
            let tx_hash = format!("{:#x}", result.transaction_hash);
            let update = DepositRequestUpdate {
                tx_hash: Some(tx_hash.clone()),
                status: DepositRequestStatus::Submitted.as_str().to_string(),
                error_code: None,
                error_message: None,
                updated_at: Some(Utc::now()),
            };

            conn.interact({
                let update = update.clone();
                let request_id = request_id.clone();
                move |conn| DepositRequest::update_status(&request_id, &update, conn)
            })
            .await
            .map_err(|e| {
                tracing::error!("Failed to update deposit request status: {e}");
                ApiError::InternalServerError
            })?
            .map_err(|e| {
                tracing::error!("Database error while updating deposit request: {e}");
                ApiError::InternalServerError
            })?;

            let response = VaultDepositResponse {
                request_id,
                status: VaultDepositStatus::Submitted,
                tx_hash: Some(tx_hash),
                referral_code,
            };

            Ok(Json(ApiResponse::ok(response)))
        }
        Err(error) => {
            let error_message = error.to_string();
            tracing::error!(
                request_id = %request_id,
                "Failed to submit Starknet deposit transaction: {error_message}"
            );

            let update = DepositRequestUpdate {
                tx_hash: None,
                status: DepositRequestStatus::Failed.as_str().to_string(),
                error_code: Some("provider_error".to_string()),
                error_message: Some(error_message.clone()),
                updated_at: Some(Utc::now()),
            };

            match conn
                .interact({
                    let update = update.clone();
                    let request_id = request_id.clone();
                    move |conn| DepositRequest::update_status(&request_id, &update, conn)
                })
                .await
            {
                Ok(Ok(_)) => {}
                Ok(Err(db_err)) => {
                    tracing::error!(
                        request_id = %request_id,
                        "Database error while persisting failed deposit status: {db_err}"
                    );
                }
                Err(pool_err) => {
                    tracing::error!(
                        request_id = %request_id,
                        "Connection error while persisting failed deposit status: {pool_err}"
                    );
                }
            }

            Err(ApiError::ServiceUnavailable(format!(
                "Failed to submit transaction (request_id: {request_id})"
            )))
        }
    }
}
