use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::schema::user_portfolio_history;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable)]
#[diesel(table_name = user_portfolio_history)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserPortfolioHistory {
    pub id: i32,
    pub user_address: String,
    pub vault_id: String,
    pub portfolio_value: Decimal,
    pub share_balance: Decimal,
    pub share_price: Decimal,
    pub calculated_at: DateTime<Utc>,
    pub created_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = user_portfolio_history)]
pub struct NewUserPortfolioHistory {
    pub user_address: String,
    pub vault_id: String,
    pub portfolio_value: Decimal,
    pub share_balance: Decimal,
    pub share_price: Decimal,
    pub calculated_at: DateTime<Utc>,
}

impl UserPortfolioHistory {
    /// Create a new portfolio history record
    pub fn create(
        new_record: &NewUserPortfolioHistory,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        diesel::insert_into(user_portfolio_history::table)
            .values(new_record)
            .get_result(conn)
    }

    /// Find portfolio history for a specific user and vault
    pub fn find_by_user_and_vault(
        user_address: &str,
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_portfolio_history::table
            .filter(user_portfolio_history::user_address.eq(user_address))
            .filter(user_portfolio_history::vault_id.eq(vault_id))
            .order(user_portfolio_history::calculated_at.asc())
            .load(conn)
    }

    /// Find portfolio history for a specific user and vault with date range
    pub fn find_by_user_vault_and_date_range(
        user_address: &str,
        vault_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_portfolio_history::table
            .filter(user_portfolio_history::user_address.eq(user_address))
            .filter(user_portfolio_history::vault_id.eq(vault_id))
            .filter(user_portfolio_history::calculated_at.ge(start_date))
            .filter(user_portfolio_history::calculated_at.le(end_date))
            .order(user_portfolio_history::calculated_at.asc())
            .load(conn)
    }

    /// Get portfolio history as time series data for risk calculations
    /// Returns a vector of (timestamp, `portfolio_value`) tuples
    pub fn get_portfolio_time_series(
        user_address: &str,
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<(DateTime<Utc>, Decimal)>> {
        let records = Self::find_by_user_and_vault(user_address, vault_id, conn)?;

        Ok(records
            .into_iter()
            .map(|record| (record.calculated_at, record.portfolio_value))
            .collect())
    }

    /// Get portfolio history as time series data with date range
    pub fn get_portfolio_time_series_with_range(
        user_address: &str,
        vault_id: &str,
        start_date: DateTime<Utc>,
        end_date: DateTime<Utc>,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<(DateTime<Utc>, Decimal)>> {
        let records = Self::find_by_user_vault_and_date_range(
            user_address,
            vault_id,
            start_date,
            end_date,
            conn,
        )?;

        Ok(records
            .into_iter()
            .map(|record| (record.calculated_at, record.portfolio_value))
            .collect())
    }

    /// Insert a portfolio history record
    /// For now, we'll allow multiple records per day and handle deduplication in the application layer
    pub fn insert_daily_record(
        user_address: &str,
        vault_id: &str,
        portfolio_value: Decimal,
        share_balance: Decimal,
        share_price: Decimal,
        calculated_at: DateTime<Utc>,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        let new_record = NewUserPortfolioHistory {
            user_address: user_address.to_string(),
            vault_id: vault_id.to_string(),
            portfolio_value,
            share_balance,
            share_price,
            calculated_at,
        };

        diesel::insert_into(user_portfolio_history::table)
            .values(&new_record)
            .get_result(conn)
    }

    /// Delete old portfolio history records (for data retention)
    pub fn delete_older_than(
        cutoff_date: DateTime<Utc>,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<usize> {
        diesel::delete(
            user_portfolio_history::table
                .filter(user_portfolio_history::calculated_at.lt(cutoff_date)),
        )
        .execute(conn)
    }

    /// Count portfolio history records for a user/vault
    pub fn count_by_user_and_vault(
        user_address: &str,
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<i64> {
        user_portfolio_history::table
            .filter(user_portfolio_history::user_address.eq(user_address))
            .filter(user_portfolio_history::vault_id.eq(vault_id))
            .count()
            .get_result(conn)
    }

    /// Get the latest portfolio history record for a user/vault
    pub fn find_latest_by_user_and_vault(
        user_address: &str,
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Option<Self>> {
        user_portfolio_history::table
            .filter(user_portfolio_history::user_address.eq(user_address))
            .filter(user_portfolio_history::vault_id.eq(vault_id))
            .order(user_portfolio_history::calculated_at.desc())
            .first(conn)
            .optional()
    }
}
