use rust_decimal::Decimal;

use zerod_db::models::{TransactionStatus, UserTransaction};

use crate::error::KpiError;

/// Calculate current cost basis and realized PNL using FIFO accounting
/// NOTE: transactions must be pre-sorted in chronological order (oldest first)
pub fn calculate_cost_basis_and_realized_pnl(
    transactions: &[UserTransaction],
) -> Result<(Decimal, Decimal), KpiError> {
    let mut share_balance = Decimal::ZERO;
    let mut cost_basis = Decimal::ZERO;
    let mut realized_pnl = Decimal::ZERO;

    for tx in transactions {
        if tx.status != TransactionStatus::Confirmed.as_str() {
            continue;
        }

        match tx.type_.as_str() {
            "deposit" => {
                if let Some(shares) = tx.shares_amount {
                    if shares < Decimal::ZERO {
                        return Err(KpiError::InvalidData(
                            "Shares amount cannot be negative".to_string(),
                        ));
                    }
                    share_balance += shares;
                    cost_basis += tx.amount; // NOTE: Amount should be normalized
                }
            }
            "withdraw" => {
                if let Some(shares) = tx.shares_amount
                    && share_balance > Decimal::ZERO
                {
                    if shares < Decimal::ZERO {
                        return Err(KpiError::InvalidData(
                            "Shares amount cannot be negative".to_string(),
                        ));
                    }
                    if shares > share_balance {
                        return Err(KpiError::InvalidData(
                            "Cannot withdraw more shares than available".to_string(),
                        ));
                    }
                    // Calculate average cost per share before withdrawal
                    let avg_cost_per_share = cost_basis / share_balance;

                    // Calculate cost basis of withdrawn shares
                    let withdrawn_cost_basis = shares * avg_cost_per_share;

                    // Calculate realized PnL for this withdrawal
                    let withdrawal_value = tx.amount; // NOTE: Amount should be normalized
                    realized_pnl += withdrawal_value - withdrawn_cost_basis;

                    // Update remaining position
                    share_balance -= shares;
                    cost_basis -= withdrawn_cost_basis;

                    // Ensure we don't go negative due to rounding
                    if share_balance <= Decimal::ZERO {
                        share_balance = Decimal::ZERO;
                        cost_basis = Decimal::ZERO;
                    }
                }
            }
            _ => {}
        }
    }

    Ok((cost_basis, realized_pnl))
}
