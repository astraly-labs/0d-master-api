use chrono::{DateTime, Utc};
use diesel::{dsl::exists, prelude::*, select};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::schema::vault_reports;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable)]
#[diesel(table_name = vault_reports)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct VaultReport {
    pub id: i32,
    pub tx_hash: String,
    pub block_number: i64,
    pub block_timestamp: DateTime<Utc>,
    pub vault_id: String,
    pub new_epoch: Decimal,
    pub new_handled_epoch_len: Decimal,
    pub total_supply: Decimal,
    pub total_aum: Decimal,
    pub management_fee_shares: Decimal,
    pub performance_fee_shares: Decimal,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = vault_reports)]
pub struct NewVaultReport {
    pub tx_hash: String,
    pub block_number: i64,
    pub block_timestamp: DateTime<Utc>,
    pub vault_id: String,
    pub new_epoch: Decimal,
    pub new_handled_epoch_len: Decimal,
    pub total_supply: Decimal,
    pub total_aum: Decimal,
    pub management_fee_shares: Decimal,
    pub performance_fee_shares: Decimal,
}

impl VaultReport {
    /// Check if a report exists by transaction hash
    pub fn exists_by_hash(tx_hash: &str, conn: &mut diesel::PgConnection) -> QueryResult<bool> {
        select(exists(
            vault_reports::table.filter(vault_reports::tx_hash.eq(tx_hash)),
        ))
        .get_result(conn)
    }

    /// Find all reports for a specific vault
    pub fn find_by_vault(
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        vault_reports::table
            .filter(vault_reports::vault_id.eq(vault_id))
            .order(vault_reports::block_timestamp.desc())
            .load(conn)
    }

    /// Find reports for a vault with pagination
    pub fn find_by_vault_paginated(
        vault_id: &str,
        limit: i64,
        offset: i64,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        vault_reports::table
            .filter(vault_reports::vault_id.eq(vault_id))
            .order(vault_reports::block_timestamp.desc())
            .limit(limit)
            .offset(offset)
            .load(conn)
    }

    /// Find the latest report for a vault
    pub fn find_latest_by_vault(
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        vault_reports::table
            .filter(vault_reports::vault_id.eq(vault_id))
            .order(vault_reports::block_timestamp.desc())
            .first(conn)
    }

    /// Create a new vault report
    pub fn create(new_report: &NewVaultReport, conn: &mut diesel::PgConnection) -> QueryResult<Self> {
        diesel::insert_into(vault_reports::table)
            .values(new_report)
            .get_result(conn)
    }

    /// Find reports within a specific epoch range for a vault
    pub fn find_by_vault_and_epoch_range(
        vault_id: &str,
        start_epoch: Decimal,
        end_epoch: Decimal,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        vault_reports::table
            .filter(vault_reports::vault_id.eq(vault_id))
            .filter(vault_reports::new_epoch.ge(start_epoch))
            .filter(vault_reports::new_epoch.le(end_epoch))
            .order(vault_reports::new_epoch.asc())
            .load(conn)
    }
}
