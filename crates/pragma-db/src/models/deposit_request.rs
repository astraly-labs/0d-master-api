use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};
use serde_json::Value;

use crate::schema::deposit_requests;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Identifiable)]
#[diesel(table_name = deposit_requests)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DepositRequest {
    pub id: String,
    pub vault_id: String,
    pub user_address: String,
    pub amount: BigDecimal,
    pub referral_code: Option<String>,
    pub transaction: Value,
    pub tx_hash: Option<String>,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = deposit_requests)]
pub struct NewDepositRequest {
    pub id: String,
    pub vault_id: String,
    pub user_address: String,
    pub amount: BigDecimal,
    pub referral_code: Option<String>,
    pub transaction: Value,
    pub tx_hash: Option<String>,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
}

#[derive(Debug, Clone, AsChangeset)]
#[diesel(table_name = deposit_requests)]
pub struct DepositRequestUpdate {
    pub tx_hash: Option<String>,
    pub status: String,
    pub error_code: Option<String>,
    pub error_message: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum DepositRequestStatus {
    Pending,
    Submitted,
    Failed,
}

impl DepositRequestStatus {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Submitted => "submitted",
            Self::Failed => "failed",
        }
    }
}

impl DepositRequest {
    pub fn create(
        new_request: &NewDepositRequest,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        diesel::insert_into(deposit_requests::table)
            .values(new_request)
            .get_result(conn)
    }

    pub fn update_status(
        id: &str,
        update: &DepositRequestUpdate,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        diesel::update(deposit_requests::table.find(id))
            .set(update)
            .get_result(conn)
    }
}
