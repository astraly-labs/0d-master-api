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
