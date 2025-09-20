use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::schema::user_kpis;
use crate::types::{PerformanceMetric, Timeframe};

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable)]
#[diesel(table_name = user_kpis)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserKpi {
    pub id: i32,
    pub user_address: String,
    pub vault_id: String,
    pub all_time_pnl: Option<Decimal>,
    pub unrealized_pnl: Option<Decimal>,
    pub realized_pnl: Option<Decimal>,
    pub max_drawdown_pct: Option<Decimal>,
    pub sharpe_ratio: Option<Decimal>,
    pub total_deposits: Option<Decimal>,
    pub total_withdrawals: Option<Decimal>,
    pub total_fees_paid: Option<Decimal>,
    pub calculated_at: Option<DateTime<Utc>>,
    pub share_price_used: Option<Decimal>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
    pub sortino_ratio: Option<Decimal>,
    pub share_balance: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = user_kpis)]
pub struct NewUserKpi {
    pub user_address: String,
    pub vault_id: String,
    pub all_time_pnl: Option<Decimal>,
    pub unrealized_pnl: Option<Decimal>,
    pub realized_pnl: Option<Decimal>,
    pub max_drawdown_pct: Option<Decimal>,
    pub sharpe_ratio: Option<Decimal>,
    pub sortino_ratio: Option<Decimal>,
    pub total_deposits: Option<Decimal>,
    pub total_withdrawals: Option<Decimal>,
    pub total_fees_paid: Option<Decimal>,
    pub calculated_at: Option<DateTime<Utc>>,
    pub share_price_used: Option<Decimal>,
    pub share_balance: Option<Decimal>,
}

#[derive(Debug, Clone, Serialize, Deserialize, AsChangeset)]
#[diesel(table_name = user_kpis)]
pub struct UserKpiUpdate {
    pub all_time_pnl: Option<Decimal>,
    pub unrealized_pnl: Option<Decimal>,
    pub realized_pnl: Option<Decimal>,
    pub max_drawdown_pct: Option<Decimal>,
    pub sharpe_ratio: Option<Decimal>,
    pub sortino_ratio: Option<Decimal>,
    pub total_deposits: Option<Decimal>,
    pub total_withdrawals: Option<Decimal>,
    pub total_fees_paid: Option<Decimal>,
    pub calculated_at: Option<DateTime<Utc>>,
    pub share_price_used: Option<Decimal>,
    pub share_balance: Option<Decimal>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl UserKpi {
    /// Get the value for a specific performance metric
    pub const fn get_metric_value(&self, metric: &PerformanceMetric) -> Option<Decimal> {
        match metric {
            PerformanceMetric::AllTimePnl => self.all_time_pnl,
            PerformanceMetric::UnrealizedPnl => self.unrealized_pnl,
            PerformanceMetric::RealizedPnl => self.realized_pnl,
        }
    }

    /// Find a KPI record by ID
    pub fn find_by_id(id: i32, conn: &mut diesel::PgConnection) -> QueryResult<Self> {
        user_kpis::table.find(id).first(conn)
    }

    /// Find all KPI records for a user
    pub fn find_by_user(
        user_address: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_kpis::table
            .filter(user_kpis::user_address.eq(user_address))
            .load(conn)
    }

    /// Find all KPI records for a vault
    pub fn find_by_vault(
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_kpis::table
            .filter(user_kpis::vault_id.eq(vault_id))
            .load(conn)
    }

    /// Find a specific user's KPIs for a vault
    pub fn find_by_user_and_vault(
        user_address: &str,
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        user_kpis::table
            .filter(user_kpis::user_address.eq(user_address))
            .filter(user_kpis::vault_id.eq(vault_id))
            .first(conn)
    }

    /// Find KPIs with positive PNL
    pub fn find_profitable(conn: &mut diesel::PgConnection) -> QueryResult<Vec<Self>> {
        user_kpis::table
            .filter(user_kpis::all_time_pnl.gt(Decimal::from(0)))
            .load(conn)
    }

    /// Find KPIs with negative PNL
    pub fn find_losing(conn: &mut diesel::PgConnection) -> QueryResult<Vec<Self>> {
        user_kpis::table
            .filter(user_kpis::all_time_pnl.lt(Decimal::from(0)))
            .load(conn)
    }

    /// Find top performers by all-time PNL
    pub fn find_top_performers(
        limit: i64,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_kpis::table
            .filter(user_kpis::all_time_pnl.is_not_null())
            .order(user_kpis::all_time_pnl.desc())
            .limit(limit)
            .load(conn)
    }

    /// Find KPIs that need recalculation (older than specified time)
    pub fn find_stale(
        cutoff_time: DateTime<Utc>,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_kpis::table
            .filter(
                user_kpis::calculated_at
                    .is_null()
                    .or(user_kpis::calculated_at.lt(cutoff_time)),
            )
            .load(conn)
    }

    /// Find recently calculated KPIs
    pub fn find_recent(
        since: DateTime<Utc>,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_kpis::table
            .filter(user_kpis::calculated_at.ge(since))
            .order(user_kpis::calculated_at.desc())
            .load(conn)
    }

    /// Create a new KPI record
    pub fn create(new_kpi: &NewUserKpi, conn: &mut diesel::PgConnection) -> QueryResult<Self> {
        diesel::insert_into(user_kpis::table)
            .values(new_kpi)
            .get_result(conn)
    }

    /// Update a KPI record
    pub fn update(
        &self,
        updates: &UserKpiUpdate,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        diesel::update(user_kpis::table.find(self.id))
            .set(updates)
            .get_result(conn)
    }

    /// Upsert (insert or update) a KPI record
    pub fn upsert(
        user_address: &str,
        vault_id: &str,
        kpi_data: &UserKpiUpdate,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        use diesel::pg::upsert::excluded;

        let new_kpi = NewUserKpi {
            user_address: user_address.to_string(),
            vault_id: vault_id.to_string(),
            all_time_pnl: kpi_data.all_time_pnl,
            unrealized_pnl: kpi_data.unrealized_pnl,
            realized_pnl: kpi_data.realized_pnl,
            max_drawdown_pct: kpi_data.max_drawdown_pct,
            sharpe_ratio: kpi_data.sharpe_ratio,
            sortino_ratio: kpi_data.sortino_ratio,
            total_deposits: kpi_data.total_deposits,
            total_withdrawals: kpi_data.total_withdrawals,
            total_fees_paid: kpi_data.total_fees_paid,
            calculated_at: kpi_data.calculated_at,
            share_price_used: kpi_data.share_price_used,
            share_balance: kpi_data.share_balance,
        };

        diesel::insert_into(user_kpis::table)
            .values(&new_kpi)
            .on_conflict((user_kpis::user_address, user_kpis::vault_id))
            .do_update()
            .set((
                user_kpis::all_time_pnl.eq(excluded(user_kpis::all_time_pnl)),
                user_kpis::unrealized_pnl.eq(excluded(user_kpis::unrealized_pnl)),
                user_kpis::realized_pnl.eq(excluded(user_kpis::realized_pnl)),
                user_kpis::max_drawdown_pct.eq(excluded(user_kpis::max_drawdown_pct)),
                user_kpis::sharpe_ratio.eq(excluded(user_kpis::sharpe_ratio)),
                user_kpis::sortino_ratio.eq(excluded(user_kpis::sortino_ratio)),
                user_kpis::total_deposits.eq(excluded(user_kpis::total_deposits)),
                user_kpis::total_withdrawals.eq(excluded(user_kpis::total_withdrawals)),
                user_kpis::total_fees_paid.eq(excluded(user_kpis::total_fees_paid)),
                user_kpis::calculated_at.eq(excluded(user_kpis::calculated_at)),
                user_kpis::share_price_used.eq(excluded(user_kpis::share_price_used)),
                user_kpis::share_balance.eq(excluded(user_kpis::share_balance)),
                user_kpis::updated_at.eq(Some(Utc::now())),
            ))
            .get_result(conn)
    }

    /// Delete a KPI record
    pub fn delete(id: i32, conn: &mut diesel::PgConnection) -> QueryResult<usize> {
        diesel::delete(user_kpis::table.find(id)).execute(conn)
    }

    /// Delete all KPIs for a user
    pub fn delete_by_user(
        user_address: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<usize> {
        diesel::delete(user_kpis::table.filter(user_kpis::user_address.eq(user_address)))
            .execute(conn)
    }

    /// Delete all KPIs for a vault
    pub fn delete_by_vault(vault_id: &str, conn: &mut diesel::PgConnection) -> QueryResult<usize> {
        diesel::delete(user_kpis::table.filter(user_kpis::vault_id.eq(vault_id))).execute(conn)
    }

    /// Count total KPI records
    pub fn count(conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        user_kpis::table.count().get_result(conn)
    }

    /// Count KPIs by user
    pub fn count_by_user(user_address: &str, conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        user_kpis::table
            .filter(user_kpis::user_address.eq(user_address))
            .count()
            .get_result(conn)
    }

    /// Count KPIs by vault
    pub fn count_by_vault(vault_id: &str, conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        user_kpis::table
            .filter(user_kpis::vault_id.eq(vault_id))
            .count()
            .get_result(conn)
    }

    /// Count profitable users
    pub fn count_profitable(conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        user_kpis::table
            .filter(user_kpis::all_time_pnl.gt(Decimal::from(0)))
            .count()
            .get_result(conn)
    }

    /// Count losing users
    pub fn count_losing(conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        user_kpis::table
            .filter(user_kpis::all_time_pnl.lt(Decimal::from(0)))
            .count()
            .get_result(conn)
    }

    /// Calculate average all-time PNL
    pub fn average_all_time_pnl(conn: &mut diesel::PgConnection) -> QueryResult<Option<Decimal>> {
        use diesel::dsl::avg;

        user_kpis::table
            .filter(user_kpis::all_time_pnl.is_not_null())
            .select(avg(user_kpis::all_time_pnl))
            .first(conn)
    }

    /// Calculate average Sharpe ratio
    pub fn average_sharpe_ratio(conn: &mut diesel::PgConnection) -> QueryResult<Option<Decimal>> {
        use diesel::dsl::avg;

        user_kpis::table
            .filter(user_kpis::sharpe_ratio.is_not_null())
            .select(avg(user_kpis::sharpe_ratio))
            .first(conn)
    }

    /// Calculate total deposits across all users
    pub fn total_deposits(conn: &mut diesel::PgConnection) -> QueryResult<Option<Decimal>> {
        use diesel::dsl::sum;

        user_kpis::table
            .filter(user_kpis::total_deposits.is_not_null())
            .select(sum(user_kpis::total_deposits))
            .first(conn)
    }

    /// Calculate total withdrawals across all users
    pub fn total_withdrawals(conn: &mut diesel::PgConnection) -> QueryResult<Option<Decimal>> {
        use diesel::dsl::sum;

        user_kpis::table
            .filter(user_kpis::total_withdrawals.is_not_null())
            .select(sum(user_kpis::total_withdrawals))
            .first(conn)
    }

    /// Calculate total fees paid across all users
    pub fn total_fees_paid(conn: &mut diesel::PgConnection) -> QueryResult<Option<Decimal>> {
        use diesel::dsl::sum;

        user_kpis::table
            .filter(user_kpis::total_fees_paid.is_not_null())
            .select(sum(user_kpis::total_fees_paid))
            .first(conn)
    }

    /// Get historical portfolio values for calculating risk metrics
    /// Returns a time series of (timestamp, `portfolio_value`) tuples
    pub fn get_portfolio_history(
        user_address: &str,
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<(DateTime<Utc>, Decimal)>> {
        // Get historical KPI records (portfolio snapshots)
        let historical_kpis = user_kpis::table
            .filter(user_kpis::user_address.eq(user_address))
            .filter(user_kpis::vault_id.eq(vault_id))
            .filter(user_kpis::calculated_at.is_not_null())
            .filter(user_kpis::share_price_used.is_not_null())
            .filter(user_kpis::share_balance.is_not_null())
            .order(user_kpis::calculated_at.asc())
            .load::<Self>(conn)?;

        // Convert KPI records to portfolio value time series
        let portfolio_history = historical_kpis
            .into_iter()
            .filter_map(
                |kpi| match (kpi.calculated_at, kpi.share_price_used, kpi.share_balance) {
                    (Some(calculated_at), Some(share_price), Some(share_balance)) => {
                        Some((calculated_at, share_balance * share_price))
                    }
                    _ => None,
                },
            )
            .collect();

        Ok(portfolio_history)
    }

    /// Get historical performance data for a specific metric and timeframe
    /// Returns time series data for `all_time_pnl`, `unrealized_pnl`, or `realized_pnl`
    pub fn get_historical_performance(
        user_address: &str,
        vault_id: &str,
        metric: &PerformanceMetric,
        timeframe: &Timeframe,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<(DateTime<Utc>, Decimal)>> {
        use chrono::Duration;

        // Calculate date filter based on timeframe
        let since_date = timeframe
            .to_days()
            .map(|days| Utc::now() - Duration::days(days));

        // Build query with optional date filter
        let mut query = user_kpis::table
            .filter(user_kpis::user_address.eq(user_address))
            .filter(user_kpis::vault_id.eq(vault_id))
            .filter(user_kpis::calculated_at.is_not_null())
            .into_boxed();

        if let Some(since) = since_date {
            query = query.filter(user_kpis::calculated_at.ge(since));
        }

        let historical_kpis = query
            .order(user_kpis::calculated_at.asc())
            .load::<Self>(conn)?;

        // Extract the requested metric from KPI records
        let historical_data = historical_kpis
            .into_iter()
            .filter_map(|kpi| {
                kpi.calculated_at.and_then(|timestamp| {
                    kpi.get_metric_value(metric).map(|value| (timestamp, value))
                })
            })
            .collect();

        Ok(historical_data)
    }
}
