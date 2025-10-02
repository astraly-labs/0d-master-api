pub mod cost_basis;
pub mod drawdown;
pub mod error;
pub mod service;
pub mod sharpe;
pub mod sortino;
pub mod task;

use chrono::{DateTime, Utc};
use rust_decimal::{Decimal, dec};

use zerod_db::models::{UserPosition, UserTransaction};

pub use cost_basis::calculate_cost_basis_and_realized_pnl;
pub use drawdown::calculate_max_drawdown;
pub use error::KpiError;
pub use service::KpiService;
pub use sharpe::calculate_sharpe_ratio;
pub use sortino::calculate_sortino_ratio;
pub use task::KpiTask;

#[derive(Debug, Clone, Default)]
pub struct PnlCalculationResult {
    pub all_time_pnl: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
}

pub fn calculate_user_pnl(
    position: &UserPosition,
    transactions: &[UserTransaction],
    current_share_price: Decimal,
) -> Result<PnlCalculationResult, KpiError> {
    debug_assert!(
        transactions
            .windows(2)
            .all(|w| w[0].block_timestamp <= w[1].block_timestamp),
        "Transactions must be sorted in chronological order"
    );

    if current_share_price <= Decimal::ZERO {
        return Err(KpiError::InvalidData(
            "Current share price cannot be zero or negative".to_string(),
        ));
    }

    if position.share_balance < Decimal::ZERO {
        return Err(KpiError::InvalidData(
            "Share balance cannot be negative".to_string(),
        ));
    }

    if position.share_balance == Decimal::ZERO {
        return Ok(PnlCalculationResult::default());
    }

    // Calculate cost basis and realized PnL from transactions
    let (current_cost_basis, realized_pnl) = calculate_cost_basis_and_realized_pnl(transactions)?;

    // Calculate unrealized PnL
    let current_position_value = position.share_balance * current_share_price;
    let unrealized_pnl = current_position_value - current_cost_basis;

    // Calculate all-time PnL
    let all_time_pnl = realized_pnl + unrealized_pnl;

    Ok(PnlCalculationResult {
        all_time_pnl,
        unrealized_pnl,
        realized_pnl,
    })
}

#[derive(Debug, Clone, Default)]
pub struct RiskMetricsResult {
    pub max_drawdown_pct: Decimal,
    pub sharpe_ratio: Decimal,
    pub sortino_ratio: Decimal,
}

pub fn calculate_risk_metrics(
    portfolio_history: &[(DateTime<Utc>, Decimal)],
) -> Result<RiskMetricsResult, KpiError> {
    if portfolio_history.len() < 2 {
        return Ok(RiskMetricsResult::default());
    }

    // TODO: Check for risk related values relevant to the vault
    let risk_free_rate = dec!(0.05); // 5% annualized
    let daily_risk_free_rate = risk_free_rate / dec!(365);

    let max_drawdown = calculate_max_drawdown(portfolio_history)?;
    let sharpe = calculate_sharpe_ratio(portfolio_history, risk_free_rate)?;
    let sortino = calculate_sortino_ratio(portfolio_history, daily_risk_free_rate)?;

    Ok(RiskMetricsResult {
        max_drawdown_pct: max_drawdown,
        sharpe_ratio: sharpe,
        sortino_ratio: sortino,
    })
}
