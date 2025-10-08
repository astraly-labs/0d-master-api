use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use std::fmt;

use crate::schema::deposit_intents;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable)]
#[diesel(table_name = deposit_intents)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct DepositIntent {
    pub id: String,
    pub partner_id: String,
    pub vault_id: String,
    pub chain_id: i64,
    pub receiver: String,
    pub amount_dec: Decimal,
    pub created_ts: DateTime<Utc>,
    pub expires_ts: DateTime<Utc>,
    pub status: String,
    pub meta_json: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = deposit_intents)]
pub struct NewDepositIntent {
    pub id: String,
    pub partner_id: String,
    pub vault_id: String,
    pub chain_id: i64,
    pub receiver: String,
    pub amount_dec: Decimal,
    pub expires_ts: DateTime<Utc>,
    pub meta_json: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, AsChangeset)]
#[diesel(table_name = deposit_intents)]
pub struct DepositIntentStatusUpdate {
    pub status: String,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum DepositIntentStatus {
    Pending,
    Matched,
    Expired,
    Orphan,
}

impl DepositIntentStatus {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Matched => "matched",
            Self::Expired => "expired",
            Self::Orphan => "orphan",
        }
    }
}

impl fmt::Display for DepositIntentStatus {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for DepositIntentStatus {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "pending" => Ok(Self::Pending),
            "matched" => Ok(Self::Matched),
            "expired" => Ok(Self::Expired),
            "orphan" => Ok(Self::Orphan),
            _ => Err("invalid deposit intent status"),
        }
    }
}

impl DepositIntent {
    pub fn find_by_id(id: &str, conn: &mut diesel::PgConnection) -> QueryResult<Self> {
        deposit_intents::table.find(id).first(conn)
    }

    pub fn create(
        new_intent: &NewDepositIntent,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        diesel::insert_into(deposit_intents::table)
            .values(new_intent)
            .get_result(conn)
    }

    pub fn update_status(
        id: &str,
        status: DepositIntentStatus,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        diesel::update(deposit_intents::table.find(id))
            .set(DepositIntentStatusUpdate {
                status: status.as_str().to_string(),
            })
            .get_result(conn)
    }
}
