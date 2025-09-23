use bigdecimal::BigDecimal;
use chrono::{DateTime, NaiveDate, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::vaults;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable)]
#[diesel(table_name = vaults)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct Vault {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub chain: String,
    pub chain_id: Option<String>,
    pub symbol: String,
    pub base_asset: String,
    pub status: String,
    pub inception_date: Option<NaiveDate>,
    pub contract_address: String,
    pub mgmt_fee_bps: Option<i32>,
    pub perf_fee_bps: i32,
    pub strategy_brief: Option<String>,
    pub docs_url: Option<String>,
    pub min_deposit: Option<BigDecimal>,
    pub max_deposit: Option<BigDecimal>,
    pub deposit_paused: Option<bool>,
    pub instant_liquidity: Option<bool>,
    pub instant_slippage_max_bps: Option<i32>,
    pub redeem_24h_threshold_pct_of_aum: Option<BigDecimal>,
    pub redeem_48h_above_threshold: Option<bool>,
    pub icon_light_url: Option<String>,
    pub icon_dark_url: Option<String>,
    pub api_endpoint: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub start_block: i64,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = vaults)]
pub struct NewVault {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub chain: String,
    pub chain_id: Option<String>,
    pub symbol: String,
    pub base_asset: String,
    pub status: String,
    pub inception_date: Option<NaiveDate>,
    pub contract_address: String,
    pub mgmt_fee_bps: Option<i32>,
    pub perf_fee_bps: i32,
    pub strategy_brief: Option<String>,
    pub docs_url: Option<String>,
    pub min_deposit: Option<BigDecimal>,
    pub max_deposit: Option<BigDecimal>,
    pub deposit_paused: Option<bool>,
    pub instant_liquidity: Option<bool>,
    pub instant_slippage_max_bps: Option<i32>,
    pub redeem_24h_threshold_pct_of_aum: Option<BigDecimal>,
    pub redeem_48h_above_threshold: Option<bool>,
    pub icon_light_url: Option<String>,
    pub icon_dark_url: Option<String>,
    pub api_endpoint: String,
    pub start_block: i64,
}

impl Vault {
    pub fn find_by_id(id: &str, conn: &mut diesel::PgConnection) -> QueryResult<Self> {
        vaults::table.find(id).first(conn)
    }

    pub fn find_all(conn: &mut diesel::PgConnection) -> QueryResult<Vec<Self>> {
        vaults::table.load(conn)
    }

    pub fn find_by_chain(chain: &str, conn: &mut diesel::PgConnection) -> QueryResult<Vec<Self>> {
        vaults::table.filter(vaults::chain.eq(chain)).load(conn)
    }

    pub fn find_live(conn: &mut diesel::PgConnection) -> QueryResult<Vec<Self>> {
        vaults::table.filter(vaults::status.eq("live")).load(conn)
    }

    /// Create a new vault
    pub fn create(new_vault: &NewVault, conn: &mut diesel::PgConnection) -> QueryResult<Self> {
        diesel::insert_into(vaults::table)
            .values(new_vault)
            .get_result(conn)
    }
}
