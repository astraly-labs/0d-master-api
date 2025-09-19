use chrono::{DateTime, Utc};
use rust_decimal::{Decimal, MathematicalOps, prelude::FromPrimitive};

use crate::error::KpiError;

// NOTE: portfolio_history must be pre-sorted in chronological order (oldest first)
pub fn calculate_sharpe_ratio(
    portfolio_history: &[(DateTime<Utc>, Decimal)],
    risk_free_rate: Decimal,
) -> Result<Decimal, KpiError> {
    if portfolio_history.len() < 2 {
        return Ok(Decimal::ZERO);
    }

    let mut prev_value = portfolio_history[0].1;
    if prev_value < Decimal::ZERO {
        return Err(KpiError::InvalidData(
            "Portfolio value cannot be negative".to_string(),
        ));
    }

    let mut sum_returns = Decimal::ZERO;
    let mut sum_squared_returns = Decimal::ZERO;
    let mut count = 0;

    for (_, value) in portfolio_history.iter().skip(1) {
        if *value < Decimal::ZERO {
            return Err(KpiError::InvalidData(
                "Portfolio value cannot be negative".to_string(),
            ));
        }
        let return_rate = if prev_value == Decimal::ZERO {
            return Err(KpiError::InvalidData(
                "Previous portfolio value is zero, cannot calculate return".to_string(),
            ));
        } else {
            (*value - prev_value) / prev_value
        };
        sum_returns += return_rate;
        sum_squared_returns += return_rate * return_rate;
        count += 1;
        prev_value = *value;
    }

    if count == 0 {
        return Ok(Decimal::ZERO);
    }

    let count_dec = Decimal::from(count);
    let mean_return = sum_returns / count_dec;
    // Variance = E[X^2] - (E[X])^2
    let variance = (sum_squared_returns / count_dec) - (mean_return * mean_return);
    let std_dev = if variance > Decimal::ZERO {
        variance.sqrt().ok_or_else(|| {
            KpiError::CalculationError("Failed to compute standard deviation".to_string())
        })?
    } else {
        return Ok(Decimal::ZERO); // No volatility, Sharpe ratio is undefined
    };

    // Assuming daily returns; adjust for actual time deltas if needed
    let annualized_return = mean_return * Decimal::from(365);
    let annualized_std_dev = std_dev
        * Decimal::from_f64(365.0_f64.sqrt()).ok_or_else(|| {
            KpiError::CalculationError("Failed to compute annualized std dev".to_string())
        })?;

    Ok((annualized_return - risk_free_rate) / annualized_std_dev)
}
