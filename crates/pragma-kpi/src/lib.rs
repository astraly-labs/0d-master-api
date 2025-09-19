pub mod cost_basis;
pub mod drawdown;
pub mod error;
pub mod service;
pub mod sharpe;
pub mod sortino;
pub mod task;

use rust_decimal::Decimal;

use pragma_db::models::{UserPosition, UserTransaction};

pub use cost_basis::calculate_cost_basis_and_realized_pnl;
pub use drawdown::calculate_max_drawdown;
pub use error::KpiError;
pub use service::KpiService;
pub use sharpe::calculate_sharpe_ratio;
pub use sortino::calculate_sortino_ratio;
pub use task::KpiTask;

#[derive(Debug, Clone, Default)]
pub struct KpiCalculationResult {
    pub all_time_pnl: Decimal,
    pub unrealized_pnl: Decimal,
    pub realized_pnl: Decimal,
    pub max_drawdown_pct: f64,
    pub sharpe_ratio: f64,
    pub sortino_ratio: f64,
}

/// Calculate user KPIs from fresh data and return calculation results
/// NOTE: transactions must be pre-sorted in chronological order (oldest first)
pub fn calculate_user_kpis(
    position: &UserPosition,
    transactions: &[UserTransaction],
    current_share_price: Decimal,
) -> Result<KpiCalculationResult, KpiError> {
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
        return Ok(KpiCalculationResult::default());
    }

    // Calculate cost basis and realized PnL from transactions
    let (current_cost_basis, realized_pnl) = calculate_cost_basis_and_realized_pnl(transactions)?;

    // Calculate unrealized PnL
    let current_position_value = position.share_balance * current_share_price;
    let unrealized_pnl = current_position_value - current_cost_basis;

    // Calculate all-time PnL
    let all_time_pnl = realized_pnl + unrealized_pnl;

    // let max_drawdown_pct = calculate_max_drawdown()?;

    // let sharpe_ratio = calculate_sharpe_ratio()?;

    // let sortino_ratio = calculate_sortino_ratio()?;

    Ok(KpiCalculationResult {
        all_time_pnl,
        unrealized_pnl,
        realized_pnl,
        max_drawdown_pct: 0.0,
        sharpe_ratio: 0.0,
        sortino_ratio: 0.0,
    })
}
