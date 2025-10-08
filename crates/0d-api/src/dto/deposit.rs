use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CreateDepositIntentRequest {
    pub chain_id: i64,
    pub receiver: String,
    pub amount: String,
    pub partner_id: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    #[serde(default)]
    pub meta: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DepositIntentResponse {
    pub intent_id: String,
    pub status: String,
    pub vault_id: String,
    pub chain_id: i64,
    pub receiver: String,
    pub amount: String,
    pub partner_id: String,
    pub created_ts: DateTime<Utc>,
    pub expires_ts: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub meta: Option<JsonValue>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub matched_tx_hash: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub confidence: Option<String>,
}
