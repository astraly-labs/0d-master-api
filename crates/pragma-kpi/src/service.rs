use anyhow::Result;
use chrono::Utc;
use deadpool_diesel::postgres::Pool;
use rust_decimal::Decimal;
use std::time::Duration;

use crate::{
    calculate_max_drawdown, calculate_sharpe_ratio, calculate_sortino_ratio, calculate_user_kpis,
};
use pragma_db::models::{
    NewUserKpi, TransactionType, UserKpi, UserKpiUpdate, UserPosition, UserTransaction, Vault,
};

pub struct KpiService {
    db_pool: Pool,
}

impl KpiService {
    const CALCULATION_INTERVAL: u64 = 24 * 60 * 60; // 24 hours

    pub const fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }

    pub async fn run_forever(&self) -> Result<()> {
        // TODO: Wait for the indexer to be fully synced (or fixed hour run)
        tokio::time::sleep(Duration::from_secs(20)).await;

        loop {
            if let Err(e) = self.run_daily_kpi_calculations().await {
                tracing::error!("[KpiService] 🔴 Error in daily KPI calculation cycle: {e}");
            }

            // Sleep before next run (24 hours)
            tokio::time::sleep(Duration::from_secs(Self::CALCULATION_INTERVAL)).await;
        }
    }

    pub async fn run_daily_kpi_calculations(&self) -> Result<()> {
        tracing::info!("[KpiService] 🧮 Starting daily KPI calculations...");
        let start_time = Utc::now();

        // Get all active vaults
        let vaults = self.get_active_vaults().await?;

        let mut total_updates = 0;
        let mut total_errors = 0;

        for vault in vaults {
            match self.calculate_vault_daily_kpis(&vault).await {
                Ok(updates) => {
                    total_updates += updates;
                }
                Err(e) => {
                    tracing::error!(
                        "[KpiService] 🔴 Failed to calculate daily KPIs for vault {}: {}",
                        vault.id,
                        e
                    );
                    total_errors += 1;
                }
            }
        }

        let duration = Utc::now() - start_time;
        tracing::info!(
            "[KpiService] 🧮 Daily KPI calculation completed in {}s. Updates: {}, Errors: {}",
            duration.num_seconds(),
            total_updates,
            total_errors
        );

        Ok(())
    }

    /// Calculate daily KPIs for all users in a specific vault
    async fn calculate_vault_daily_kpis(&self, vault: &Vault) -> Result<usize> {
        // Get current share price from vault API
        let current_share_price = Self::fetch_vault_share_price(vault).await?;

        // Get all users with positions in this vault
        let user_positions = self.get_vault_user_positions(&vault.id).await?;

        let mut updated_count = 0;

        for position in user_positions {
            match self
                .calculate_user_daily_kpis(&position, &vault.id, current_share_price)
                .await
            {
                Ok(()) => updated_count += 1,
                Err(e) => {
                    tracing::error!(
                        "[KpiService] 🔴 Failed to calculate daily KPIs for user {} in vault {}: {:?}",
                        position.user_address,
                        vault.id,
                        e
                    );
                }
            }
        }

        Ok(updated_count)
    }

    /// Calculate and store ALL daily KPIs for a specific user in a specific vault
    async fn calculate_user_daily_kpis(
        &self,
        position: &UserPosition,
        vault_id: &str,
        current_share_price: Decimal,
    ) -> Result<()> {
        // Get user transactions for this vault
        let transactions = self
            .get_user_vault_transactions(&position.user_address, vault_id)
            .await?;

        // Calculate basic KPIs (PnL, deposits, withdrawals, fees)
        let basic_kpis = calculate_user_kpis(position, &transactions, current_share_price)?;

        // Get historical portfolio data for risk metrics
        let portfolio_history = self
            .get_user_portfolio_history(&position.user_address, vault_id, current_share_price)
            .await?;

        // Calculate risk metrics using historical data
        let (max_drawdown_pct, sharpe_ratio, sortino_ratio) = if portfolio_history.len() >= 2 {
            // TODO: Check for risk related values relevant to the vault
            let risk_free_rate = Decimal::from(5) / Decimal::from(100); // 5% annualized
            let daily_risk_free_rate = risk_free_rate / Decimal::from(365);

            let max_drawdown = calculate_max_drawdown(&portfolio_history)?;
            let sharpe = calculate_sharpe_ratio(&portfolio_history, risk_free_rate)?;
            let sortino = calculate_sortino_ratio(&portfolio_history, daily_risk_free_rate)?;

            (max_drawdown, sharpe, sortino)
        } else {
            (Decimal::ZERO, Decimal::ZERO, Decimal::ZERO)
        };

        // Create comprehensive KPI update
        let kpi_update = UserKpiUpdate {
            all_time_pnl: Some(basic_kpis.all_time_pnl),
            unrealized_pnl: Some(basic_kpis.unrealized_pnl),
            realized_pnl: Some(basic_kpis.realized_pnl),
            max_drawdown_pct: Some(max_drawdown_pct),
            sharpe_ratio: Some(sharpe_ratio),
            sortino_ratio: Some(sortino_ratio),
            total_deposits: Some(Self::calculate_total_deposits(&transactions)),
            total_withdrawals: Some(Self::calculate_total_withdrawals(&transactions)),
            total_fees_paid: Some(Self::calculate_total_fees(&transactions)),
            calculated_at: Some(Utc::now()),
            share_price_used: Some(current_share_price),
            share_balance: Some(position.share_balance), // Store current share balance
            updated_at: Some(Utc::now()),
        };

        // Insert daily KPI record (create new record each day)
        self.insert_daily_user_kpis(&position.user_address, vault_id, &kpi_update)
            .await?;

        Ok(())
    }

    /// Get historical portfolio values for calculating risk metrics
    /// Returns a time series of (timestamp, `portfolio_value`) tuples
    async fn get_user_portfolio_history(
        &self,
        user_address: &str,
        vault_id: &str,
        current_share_price: Decimal,
    ) -> Result<Vec<(chrono::DateTime<Utc>, Decimal)>> {
        let conn = self.db_pool.get().await?;
        let user_address_clone = user_address.to_string();
        let vault_id_clone = vault_id.to_string();

        // Get historical portfolio data from model
        let mut portfolio_history = conn
            .interact(move |conn| {
                UserKpi::get_portfolio_history(&user_address_clone, &vault_id_clone, conn)
            })
            .await
            .map_err(|e| {
                anyhow::anyhow!("[KpiService] 🗃️ Database interaction error: {:?}", e)
            })??;

        // Add current portfolio value if we have historical data
        if !portfolio_history.is_empty() {
            let current_position = self.get_user_position(user_address, vault_id).await?;
            let current_portfolio_value = current_position.share_balance * current_share_price;
            portfolio_history.push((Utc::now(), current_portfolio_value));
        }

        Ok(portfolio_history)
    }

    /// Calculate total deposits from transactions
    fn calculate_total_deposits(transactions: &[UserTransaction]) -> Decimal {
        transactions
            .iter()
            .filter(|tx| tx.type_ == TransactionType::Deposit.as_str())
            .map(|tx| tx.amount)
            .sum()
    }

    /// Calculate total withdrawals from transactions
    fn calculate_total_withdrawals(transactions: &[UserTransaction]) -> Decimal {
        transactions
            .iter()
            .filter(|tx| tx.type_ == TransactionType::Withdraw.as_str())
            .map(|tx| tx.amount)
            .sum()
    }

    /// Calculate total fees paid from transactions
    /// Currently calculates gas fees. Management/performance fees may need separate tracking.
    fn calculate_total_fees(transactions: &[UserTransaction]) -> Decimal {
        transactions.iter().filter_map(|tx| tx.gas_fee).sum()
    }

    /// Get all active vaults
    async fn get_active_vaults(&self) -> Result<Vec<Vault>> {
        let conn = self.db_pool.get().await?;

        let vaults = conn
            .interact(Vault::find_live)
            .await
            .map_err(|e| anyhow::anyhow!("Database interaction error: {:?}", e))??;

        Ok(vaults)
    }

    /// Get all user positions for a vault
    async fn get_vault_user_positions(&self, vault_id: &str) -> Result<Vec<UserPosition>> {
        let conn = self.db_pool.get().await?;
        let vault_id_clone = vault_id.to_string();

        let positions = conn
            .interact(move |conn| UserPosition::find_active_by_vault(&vault_id_clone, conn))
            .await
            .map_err(|e| anyhow::anyhow!("Database interaction error: {:?}", e))??;

        Ok(positions)
    }

    /// Get user transactions for a specific vault
    async fn get_user_vault_transactions(
        &self,
        user_address: &str,
        vault_id: &str,
    ) -> Result<Vec<UserTransaction>> {
        let conn = self.db_pool.get().await?;
        let user_address = user_address.to_string();
        let vault_id = vault_id.to_string();

        let transactions = conn
            .interact(move |conn| {
                UserTransaction::find_by_user_and_vault_chronological(
                    &user_address,
                    &vault_id,
                    conn,
                )
            })
            .await
            .map_err(|e| {
                anyhow::anyhow!("[KpiService] 🗃️ Database interaction error: {:?}", e)
            })??;

        Ok(transactions)
    }

    /// Insert new daily user KPI record (creates historical records for portfolio analysis)
    async fn insert_daily_user_kpis(
        &self,
        user_address: &str,
        vault_id: &str,
        kpi_data: &UserKpiUpdate,
    ) -> Result<()> {
        let conn = self.db_pool.get().await?;
        let user_address = user_address.to_string();
        let vault_id = vault_id.to_string();
        let kpi_data = kpi_data.clone();

        conn.interact(move |conn| {
            let new_kpi = NewUserKpi {
                user_address,
                vault_id,
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

            UserKpi::create(&new_kpi, conn)
        })
        .await
        .map_err(|e| anyhow::anyhow!("[KpiService] 🗃️ Database interaction error: {:?}", e))??;

        Ok(())
    }

    /// Get user position
    async fn get_user_position(&self, user_address: &str, vault_id: &str) -> Result<UserPosition> {
        let conn = self.db_pool.get().await?;
        let user_address_clone = user_address.to_string();
        let vault_id_clone = vault_id.to_string();

        let position = conn
            .interact(move |conn| {
                UserPosition::find_by_user_and_vault(&user_address_clone, &vault_id_clone, conn)
            })
            .await
            .map_err(|e| {
                anyhow::anyhow!("[KpiService] 🗃️ Database interaction error: {:?}", e)
            })??;

        Ok(position)
    }

    /// Fetch vault share price and convert to Decimal
    async fn fetch_vault_share_price(vault: &Vault) -> Result<Decimal> {
        let client = reqwest::Client::builder()
            .timeout(Duration::from_secs(5))
            .build()?;

        let url = format!("{}/nav/latest", vault.api_endpoint.trim_end_matches('/'));
        let response = client.get(&url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow::anyhow!(
                "[KpiService] 🔴 Vault API returned status: {}",
                response.status(),
            ));
        }

        let nav_data: serde_json::Value = response.json().await?;
        let share_price_str = nav_data
            .get("share_price")
            .and_then(|v| v.as_str())
            .ok_or_else(|| {
                anyhow::anyhow!("[KpiService] 🔴 Share price not found in vault API response")
            })?;

        let share_price = share_price_str.parse::<Decimal>()?;
        Ok(share_price)
    }
}
