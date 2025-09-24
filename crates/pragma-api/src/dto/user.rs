use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use pragma_db::models::user_transaction::{TransactionStatus, TransactionType};

pub use pragma_db::types::{PerformanceMetric, Timeframe};

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum DisplayCurrency {
    USD,
    USDC,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserProfile {
    pub address: String,
    pub chain: String,
    pub display_currency: DisplayCurrency,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserPositionSummary {
    pub vault_id: String,
    pub as_of: DateTime<Utc>,
    pub position_value_usd: String,
    pub share_balance: String,
    pub share_price: String,
    pub first_deposit_at: Option<DateTime<Utc>>,
    pub total_deposits: String,
    pub all_time_earned: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserTransaction {
    pub id: String,
    pub vault_id: String,
    #[serde(rename = "type")]
    pub transaction_type: TransactionType,
    pub status: TransactionStatus,
    pub amount: String,
    pub tx_hash: String,
    pub timestamp: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserTransactionHistory {
    pub items: Vec<UserTransaction>,
    pub next_cursor: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct UserKpi {
    pub as_of: DateTime<Utc>,
    pub all_time_pnl_usd: String,
    pub unrealized_pnl_usd: String,
    pub realized_pnl_usd: String,
    pub max_drawdown_pct: f64,
    pub sharpe: f64,
    pub sortino: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HistoricalDataPoint {
    pub t: DateTime<Utc>,
    pub v: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct HistoricalUserPerformance {
    pub metric: PerformanceMetric,
    pub timeframe: Timeframe,
    pub points: Vec<HistoricalDataPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingRedeem {
    pub vault_id: String,
    pub amount: String,
    pub transaction_type: TransactionType,
    pub tx_hash: String,
    pub submitted_at: DateTime<Utc>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct PendingRedeemsResponse {
    pub address: String,
    pub as_of: DateTime<Utc>,
    pub pending_redeems: Vec<PendingRedeem>,
    pub total_pending: String,
}

impl From<pragma_db::models::User> for UserProfile {
    fn from(user: pragma_db::models::User) -> Self {
        Self {
            address: user.address,
            chain: user.chain,
            display_currency: DisplayCurrency::USD,
        }
    }
}

impl From<pragma_db::models::UserTransaction> for UserTransaction {
    fn from(tx: pragma_db::models::UserTransaction) -> Self {
        Self {
            id: tx.id.to_string(),
            vault_id: tx.vault_id,
            transaction_type: match tx.type_.as_str() {
                "deposit" => TransactionType::Deposit,
                "withdraw" => TransactionType::Withdraw,
                "transfer" => TransactionType::Transfer,
                "claim" => TransactionType::Claim,
                _ => unreachable!("Invalid transaction type: {}", tx.type_),
            },
            status: match tx.status.as_str() {
                "pending" => TransactionStatus::Pending,
                "confirmed" => TransactionStatus::Confirmed,
                "failed" => TransactionStatus::Failed,
                "cancelled" => TransactionStatus::Cancelled,
                _ => unreachable!("Invalid transaction status: {}", tx.status),
            },
            amount: tx.amount.to_string(),
            tx_hash: tx.tx_hash,
            timestamp: tx.block_timestamp,
        }
    }
}

impl From<pragma_db::models::UserTransaction> for PendingRedeem {
    fn from(tx: pragma_db::models::UserTransaction) -> Self {
        Self {
            vault_id: tx.vault_id,
            amount: tx.amount.to_string(),
            transaction_type: match tx.type_.as_str() {
                "deposit" => TransactionType::Deposit,
                "withdraw" => TransactionType::Withdraw,
                "transfer" => TransactionType::Transfer,
                "claim" => TransactionType::Claim,
                _ => unreachable!("Invalid transaction type: {}", tx.type_),
            },
            tx_hash: tx.tx_hash,
            submitted_at: tx.block_timestamp,
        }
    }
}
