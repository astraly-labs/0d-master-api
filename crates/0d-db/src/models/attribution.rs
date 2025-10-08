use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use std::fmt;

use crate::schema::attributions;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable)]
#[diesel(table_name = attributions)]
#[diesel(primary_key(tx_id))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Attribution {
    pub tx_id: i32,
    pub tx_hash: String,
    pub intent_id: Option<String>,
    pub partner_id: Option<String>,
    pub source: String,
    pub confidence: Decimal,
    pub assets_dec: Decimal,
    pub shares_dec: Option<Decimal>,
    pub created_ts: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = attributions)]
pub struct NewAttribution {
    pub tx_id: i32,
    pub tx_hash: String,
    pub intent_id: Option<String>,
    pub partner_id: Option<String>,
    pub source: String,
    pub confidence: Decimal,
    pub assets_dec: Decimal,
    pub shares_dec: Option<Decimal>,
}

#[derive(Debug, Clone, Copy, Serialize, Deserialize, PartialEq, Eq)]
#[serde(rename_all = "lowercase")]
pub enum AttributionSource {
    Explicit,
    Inferred,
}

impl AttributionSource {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Explicit => "explicit",
            Self::Inferred => "inferred",
        }
    }
}

impl fmt::Display for AttributionSource {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(self.as_str())
    }
}

impl TryFrom<&str> for AttributionSource {
    type Error = &'static str;

    fn try_from(value: &str) -> Result<Self, Self::Error> {
        match value {
            "explicit" => Ok(Self::Explicit),
            "inferred" => Ok(Self::Inferred),
            _ => Err("invalid attribution source"),
        }
    }
}

impl Attribution {
    pub fn find_by_intent_id(
        intent_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Option<Self>> {
        match attributions::table
            .filter(attributions::intent_id.eq(intent_id))
            .first::<Self>(conn)
        {
            Ok(record) => Ok(Some(record)),
            Err(diesel::result::Error::NotFound) => Ok(None),
            Err(err) => Err(err),
        }
    }

    pub fn create(
        new_attribution: &NewAttribution,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        diesel::insert_into(attributions::table)
            .values(new_attribution)
            .get_result(conn)
    }
}
