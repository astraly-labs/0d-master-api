use std::str::FromStr;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{Duration, Utc};
use rust_decimal::Decimal;
use tracing::info;
use ulid::Ulid;

use crate::{
    AppState,
    dto::{ApiResponse, CreateDepositIntentRequest, DepositIntentResponse},
    errors::{ApiError, DatabaseErrorExt},
    helpers::{fetch_vault, normalize_starknet_address},
};
use zerod_db::{
    ZerodPool,
    models::{Attribution, DepositIntent, DepositIntentStatus, NewDepositIntent},
};

const DEFAULT_EXPIRY_MINUTES: i64 = 15;

#[utoipa::path(
    post,
    path = "/v1/vaults/{vault_id}/deposits/intents",
    tag = "Deposits",
    params(("vault_id" = String, Path, description = "Vault identifier")),
    request_body = CreateDepositIntentRequest,
    responses(
        (status = 200, description = "Deposit intent created", body = ApiResponse<DepositIntentResponse>),
        (status = 400, description = "Invalid request"),
        (status = 404, description = "Vault not found"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn create_deposit_intent(
    Path(vault_id): Path<String>,
    State(state): State<AppState>,
    Json(payload): Json<CreateDepositIntentRequest>,
) -> Result<impl IntoResponse, ApiError> {
    let CreateDepositIntentRequest {
        chain_id,
        receiver,
        amount,
        partner_id,
        meta,
    } = payload;

    let vault = fetch_vault(&state, &vault_id).await?;

    if let Some(expected_chain) = vault.chain_id.as_deref() {
        if let Ok(expected_id) = expected_chain.parse::<i64>() {
            if expected_id != chain_id {
                return Err(ApiError::BadRequest(format!(
                    "Chain id {chain_id} does not match vault {vault_id} (expected {expected_id})"
                )));
            }
        }
    }

    let receiver = normalize_starknet_address(&receiver)?;

    let amount_dec = Decimal::from_str(&amount)
        .map_err(|_| ApiError::BadRequest("Amount must be a valid decimal string".to_string()))?;

    if amount_dec <= Decimal::ZERO {
        return Err(ApiError::BadRequest(
            "Amount must be strictly positive".to_string(),
        ));
    }

    let amount_dec = amount_dec.normalize();
    let now = Utc::now();
    let expires_ts = now + Duration::minutes(DEFAULT_EXPIRY_MINUTES);

    let intent_id = format!("din_{}", Ulid::new());

    let new_intent = NewDepositIntent {
        id: intent_id.clone(),
        partner_id: partner_id.clone(),
        vault_id: vault_id.clone(),
        chain_id,
        receiver: receiver.clone(),
        amount_dec,
        expires_ts,
        meta_json: meta,
    };

    let created_intent = state
        .pool
        .interact_with_context(format!("create deposit intent {intent_id}"), move |conn| {
            DepositIntent::create(&new_intent, conn)
        })
        .await?;

    state.metrics.deposits.record_intent_created(
        &created_intent.vault_id,
        created_intent.chain_id,
        &created_intent.partner_id,
    );

    info!(
        intent_id = %created_intent.id,
        vault_id = %created_intent.vault_id,
        partner_id = %created_intent.partner_id,
        chain_id = created_intent.chain_id,
        receiver = %created_intent.receiver,
        "Deposit intent created",
    );

    let response = map_intent_to_dto(created_intent, None);
    Ok((StatusCode::OK, Json(ApiResponse::ok(response))))
}

#[utoipa::path(
    get,
    path = "/v1/deposits/intents/{intent_id}",
    tag = "Deposits",
    params(("intent_id" = String, Path, description = "Deposit intent identifier")),
    responses(
        (status = 200, description = "Deposit intent details", body = ApiResponse<DepositIntentResponse>),
        (status = 404, description = "Deposit intent not found"),
        (status = 500, description = "Internal server error"),
    )
)]
pub async fn get_deposit_intent(
    Path(intent_id): Path<String>,
    State(state): State<AppState>,
) -> Result<impl IntoResponse, ApiError> {
    let intent_id_clone = intent_id.clone();
    let (intent, attribution) = state
        .pool
        .interact_with_context(
            format!("fetch deposit intent {intent_id}"),
            move |conn| -> Result<_, diesel::result::Error> {
                let intent = DepositIntent::find_by_id(&intent_id_clone, conn)?;
                let attribution = Attribution::find_by_intent_id(&intent_id_clone, conn)?;
                Ok((intent, attribution))
            },
        )
        .await
        .map_err(|err| err.or_not_found(format!("Intent {intent_id} not found")))?;

    let response = map_intent_to_dto(intent, attribution);
    Ok(Json(ApiResponse::ok(response)))
}

fn map_intent_to_dto(
    intent: DepositIntent,
    attribution: Option<Attribution>,
) -> DepositIntentResponse {
    let DepositIntent {
        id,
        partner_id,
        vault_id,
        chain_id,
        receiver,
        amount_dec,
        created_ts,
        expires_ts,
        status,
        meta_json,
    } = intent;

    let status = DepositIntentStatus::try_from(status.as_str())
        .map(|s| s.as_str().to_string())
        .unwrap_or(status);

    let (matched_tx_hash, confidence) = attribution
        .map(|attr| {
            let confidence = attr.confidence.normalize().to_string();
            (Some(attr.tx_hash), Some(confidence))
        })
        .unwrap_or((None, None));

    DepositIntentResponse {
        intent_id: id,
        status,
        vault_id,
        chain_id,
        receiver,
        amount: amount_dec.normalize().to_string(),
        partner_id,
        created_ts,
        expires_ts,
        meta: meta_json,
        matched_tx_hash,
        confidence,
    }
}
