use chrono::{DateTime, Utc};
use rust_decimal::Decimal;

use crate::error::KpiError;

// NOTE: portfolio_history must be pre-sorted in chronological order (oldest first)
pub fn calculate_max_drawdown(
    portfolio_history: &[(DateTime<Utc>, Decimal)],
) -> Result<Decimal, KpiError> {
    if portfolio_history.is_empty() {
        return Ok(Decimal::ZERO);
    }

    let mut peak_value = Decimal::ZERO;
    let mut max_drawdown: Decimal = Decimal::ZERO;

    for (_, value) in portfolio_history {
        if *value < Decimal::ZERO {
            return Err(KpiError::InvalidData(
                "Portfolio value cannot be negative".to_string(),
            ));
        }
        if *value > peak_value {
            peak_value = *value;
        } else if peak_value > Decimal::ZERO {
            let drawdown = ((peak_value - *value) / peak_value) * Decimal::from(100);
            max_drawdown = max_drawdown.max(drawdown);
        }
    }

    Ok(max_drawdown)
}
