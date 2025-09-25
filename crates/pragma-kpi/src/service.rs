use chrono::Utc;
use deadpool_diesel::postgres::Pool;
use rust_decimal::Decimal;
use std::time::Duration;

use pragma_db::models::{
    IndexerState, UserKpi, UserKpiUpdate, UserPortfolioHistory, UserPosition, UserTransaction,
    Vault,
};
use pragma_master::VaultMasterAPIClient;

use crate::{calculate_risk_metrics, calculate_user_pnl};

pub struct KpiService {
    db_pool: Pool,
}

impl KpiService {
    const CALCULATION_INTERVAL: Duration = Duration::from_secs(24 * 60 * 60); // 24 hours
    const WAIT_INDEXERS_INTERVAL: Duration = Duration::from_secs(30); // 30 seconds

    pub const fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }

    pub async fn run_forever(&self) -> anyhow::Result<()> {
        // Wait for all indexers to be fully synced before starting KPI calculations
        // TODO: Chose a fix hour to run the KPI calculations
        self.wait_for_indexers_synced().await?;

        loop {
            if let Err(e) = self.run_daily_kpi_calculations().await {
                tracing::error!("[KpiService] 🔴 Error in daily KPI calculation cycle: {e}");
            }

            // Sleep before next run (24 hours)
            tokio::time::sleep(Self::CALCULATION_INTERVAL).await;
        }
    }

    /// Wait for all indexers to be fully synced before starting KPI calculations
    async fn wait_for_indexers_synced(&self) -> anyhow::Result<()> {
        tracing::info!("[KpiService] ⏳ Waiting for all indexers to be synced...");

        loop {
            tokio::time::sleep(Self::WAIT_INDEXERS_INTERVAL).await;

            let conn = self.db_pool.get().await?;

            let indexer_states = conn
                .interact(IndexerState::find_all)
                .await
                .map_err(|e| anyhow::anyhow!("[KpiService] 🗃️ Database interaction failed: {e}"))?
                .map_err(|e| {
                    anyhow::anyhow!("[KpiService] 🗃️ Failed to load indexer states: {e}")
                })?;

            if indexer_states.iter().all(IndexerState::is_synced) {
                break;
            }
        }

        Ok(())
    }

    pub async fn run_daily_kpi_calculations(&self) -> anyhow::Result<()> {
        tracing::info!("[KpiService] 🧮 Starting daily KPI calculations...");
        let start_time = Utc::now();

        let vaults = self.get_active_vaults().await?;
        let mut total_updates = 0;
        let mut total_errors = 0;

        for vault in vaults {
            match self.calculate_vault_daily_kpis(&vault).await {
                Ok(updates) => total_updates += updates,
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
    async fn calculate_vault_daily_kpis(&self, vault: &Vault) -> anyhow::Result<usize> {
        let current_share_price = Self::fetch_vault_share_price(vault).await?;
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

        tracing::info!(
            "[KpiService] 🧮 Completed KPI calculation for vault {}: {} users updated",
            vault.id,
            updated_count
        );

        Ok(updated_count)
    }

    /// Calculate and store ALL daily KPIs for a specific user in a specific vault
    async fn calculate_user_daily_kpis(
        &self,
        position: &UserPosition,
        vault_id: &str,
        current_share_price: Decimal,
    ) -> anyhow::Result<()> {
        // Get user transactions for this vault
        let transactions = self
            .get_user_vault_transactions(&position.user_address, vault_id)
            .await?;

        // Calculate PnL
        let pnl_result = calculate_user_pnl(position, &transactions, current_share_price)?;

        // Calculate current portfolio value
        let current_portfolio_value = position.share_balance * current_share_price;

        // Insert daily portfolio history snapshot
        self.insert_daily_portfolio_history(
            &position.user_address,
            vault_id,
            current_portfolio_value,
            position.share_balance,
            current_share_price,
        )
        .await?;

        // Get historical portfolio data for risk metrics from portfolio history table
        let portfolio_history = self
            .get_user_portfolio_history(&position.user_address, vault_id)
            .await?;

        // Calculate risk metrics using historical data
        let risk_metrics = calculate_risk_metrics(&portfolio_history)?;

        // Create comprehensive KPI update
        let kpi_update = UserKpiUpdate {
            all_time_pnl: Some(pnl_result.all_time_pnl),
            unrealized_pnl: Some(pnl_result.unrealized_pnl),
            realized_pnl: Some(pnl_result.realized_pnl),
            max_drawdown_pct: Some(risk_metrics.max_drawdown_pct),
            sharpe_ratio: Some(risk_metrics.sharpe_ratio),
            sortino_ratio: Some(risk_metrics.sortino_ratio),
            total_deposits: Some(UserTransaction::calculate_total_deposits(&transactions)),
            total_withdrawals: Some(UserTransaction::calculate_total_withdrawals(&transactions)),
            total_fees_paid: Some(UserTransaction::calculate_total_fees(&transactions)),
            calculated_at: Some(Utc::now()),
            share_price_used: Some(current_share_price),
            share_balance: Some(position.share_balance), // Store current share balance
            updated_at: Some(Utc::now()),
        };

        // Update current KPI record with latest values
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
    ) -> anyhow::Result<Vec<(chrono::DateTime<Utc>, Decimal)>> {
        let conn = self.db_pool.get().await?;
        let user_address_clone = user_address.to_string();
        let vault_id_clone = vault_id.to_string();

        // Get historical portfolio data from portfolio history table
        let portfolio_history = conn
            .interact(move |conn| {
                UserPortfolioHistory::get_portfolio_time_series(
                    &user_address_clone,
                    &vault_id_clone,
                    conn,
                )
            })
            .await
            .map_err(|e| anyhow::anyhow!("[KpiService] 🗃️ Database interaction error: {e}"))??;

        Ok(portfolio_history)
    }

    /// Get all active vaults
    async fn get_active_vaults(&self) -> anyhow::Result<Vec<Vault>> {
        let conn = self.db_pool.get().await?;

        let vaults = conn
            .interact(Vault::find_live)
            .await
            .map_err(|e| anyhow::anyhow!("[KpiService] 🗃️ Database error: {e}"))??;

        Ok(vaults)
    }

    /// Get all user positions for a vault
    async fn get_vault_user_positions(&self, vault_id: &str) -> anyhow::Result<Vec<UserPosition>> {
        let conn = self.db_pool.get().await?;
        let vault_id_clone = vault_id.to_string();

        let positions = conn
            .interact(move |conn| UserPosition::find_active_by_vault(&vault_id_clone, conn))
            .await
            .map_err(|e| anyhow::anyhow!("[KpiService] 🗃️ Database interaction error: {e}"))??;

        Ok(positions)
    }

    /// Get user transactions for a specific vault
    async fn get_user_vault_transactions(
        &self,
        user_address: &str,
        vault_id: &str,
    ) -> anyhow::Result<Vec<UserTransaction>> {
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
            .map_err(|e| anyhow::anyhow!("[KpiService] 🗃️ Database interaction error: {e}"))??;

        Ok(transactions)
    }

    /// Insert daily portfolio history snapshot
    async fn insert_daily_portfolio_history(
        &self,
        user_address: &str,
        vault_id: &str,
        portfolio_value: Decimal,
        share_balance: Decimal,
        share_price: Decimal,
    ) -> anyhow::Result<()> {
        let conn = self.db_pool.get().await?;
        let user_address = user_address.to_string();
        let vault_id = vault_id.to_string();
        let calculated_at = Utc::now();

        conn.interact(move |conn| {
            UserPortfolioHistory::insert_daily_record(
                &user_address,
                &vault_id,
                portfolio_value,
                share_balance,
                share_price,
                calculated_at,
                conn,
            )
        })
        .await
        .map_err(|e| anyhow::anyhow!("[KpiService] 🗃️ Database interaction error: {e}"))??;

        Ok(())
    }

    /// Insert or update daily user KPI record (upserts to handle multiple runs per day)
    async fn insert_daily_user_kpis(
        &self,
        user_address: &str,
        vault_id: &str,
        kpi_data: &UserKpiUpdate,
    ) -> anyhow::Result<()> {
        let conn = self.db_pool.get().await?;
        let user_address = user_address.to_string();
        let vault_id = vault_id.to_string();
        let kpi_data = kpi_data.clone();

        conn.interact(move |conn| UserKpi::upsert(&user_address, &vault_id, &kpi_data, conn))
            .await
            .map_err(|e| anyhow::anyhow!("[KpiService] 🗃️ Database interaction error: {e}"))??;

        Ok(())
    }

    /// Fetch vault share price using `VaultMasterAPIClient`
    async fn fetch_vault_share_price(vault: &Vault) -> anyhow::Result<Decimal> {
        let client = VaultMasterAPIClient::new(&vault.api_endpoint)
            .map_err(|e| anyhow::anyhow!("Failed to create vault client: {e}"))?;

        let share_price_str = client
            .get_vault_share_price()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to fetch share price: {e}"))?;

        share_price_str
            .parse::<Decimal>()
            .map_err(|e| anyhow::anyhow!("Invalid share price format: {e}"))
    }
}
