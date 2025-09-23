use rust_decimal::{Decimal, MathematicalOps, prelude::FromPrimitive};

use chrono::{DateTime, Utc};

use crate::error::KpiError;

// NOTE: portfolio_history must be pre-sorted in chronological order (oldest first)
pub fn calculate_sortino_ratio(
    portfolio_history: &[(DateTime<Utc>, Decimal)],
    target_return: Decimal, // Daily, e.g., risk_free_rate / 365
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
    let mut sum_downside_squared = Decimal::ZERO;
    let mut downside_count = 0;
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
        if return_rate < target_return {
            let downside = return_rate - target_return;
            sum_downside_squared += downside * downside;
            downside_count += 1;
        }
        count += 1;
        prev_value = *value;
    }

    if count == 0 {
        return Ok(Decimal::ZERO);
    }

    let count_dec = Decimal::from(count);
    let mean_return = sum_returns / count_dec;

    if downside_count == 0 {
        return if mean_return > target_return {
            Ok(Decimal::MAX) // No downside risk, return maximum representable value
        } else {
            Ok(Decimal::ZERO) // No excess return, Sortino ratio is zero
        };
    }

    let downside_count_dec = Decimal::from(downside_count);
    let downside_variance = sum_downside_squared / downside_count_dec;
    let downside_deviation = if downside_variance > Decimal::ZERO {
        downside_variance.sqrt().ok_or_else(|| {
            KpiError::CalculationError("Failed to compute downside deviation".to_string())
        })?
    } else {
        return Ok(Decimal::ZERO); // No downside volatility, Sortino ratio is undefined
    };

    // Assuming daily returns; adjust for actual time deltas if needed
    let annualized_return = mean_return * Decimal::from(365);
    let annualized_target = target_return * Decimal::from(365);
    let annualized_downside_dev = downside_deviation
        * Decimal::from_f64(365.0_f64.sqrt()).ok_or_else(|| {
            KpiError::CalculationError("Failed to compute annualized downside dev".to_string())
        })?;

    Ok((annualized_return - annualized_target) / annualized_downside_dev)
}
