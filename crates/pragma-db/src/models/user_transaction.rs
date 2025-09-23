use chrono::{DateTime, Utc};
use diesel::{dsl::exists, prelude::*, select};
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use serde_json::Value as JsonValue;
use utoipa::ToSchema;

use crate::schema::user_transactions;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable)]
#[diesel(table_name = user_transactions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserTransaction {
    pub id: i32,
    pub tx_hash: String,
    pub block_number: i64,
    pub block_timestamp: DateTime<Utc>,
    pub user_address: String,
    pub vault_id: String,
    pub type_: String,
    pub status: String,
    pub amount: Decimal,
    pub shares_amount: Option<Decimal>,
    pub share_price: Option<Decimal>,
    pub gas_fee: Option<Decimal>,
    pub metadata: Option<JsonValue>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = user_transactions)]
pub struct NewUserTransaction {
    pub tx_hash: String,
    pub block_number: i64,
    pub block_timestamp: DateTime<Utc>,
    pub user_address: String,
    pub vault_id: String,
    pub type_: String,
    pub status: String,
    pub amount: Decimal,
    pub shares_amount: Option<Decimal>,
    pub share_price: Option<Decimal>,
    pub gas_fee: Option<Decimal>,
    pub metadata: Option<JsonValue>,
}

#[derive(Debug, Clone, Serialize, Deserialize, AsChangeset)]
#[diesel(table_name = user_transactions)]
pub struct UserTransactionUpdate {
    pub status: Option<String>,
    pub shares_amount: Option<Decimal>,
    pub share_price: Option<Decimal>,
    pub gas_fee: Option<Decimal>,
    pub metadata: Option<JsonValue>,
    pub updated_at: Option<DateTime<Utc>>,
}

// Transaction types enum for better type safety
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TransactionType {
    Deposit,
    Withdraw,
    Transfer,
    Claim,
}

impl TransactionType {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Deposit => "deposit",
            Self::Withdraw => "withdraw",
            Self::Transfer => "transfer",
            Self::Claim => "claim",
        }
    }
}

// Transaction status enum for better type safety
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TransactionStatus {
    Pending,
    Confirmed,
    Failed,
    Cancelled,
}

impl TransactionStatus {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Pending => "pending",
            Self::Confirmed => "confirmed",
            Self::Failed => "failed",
            Self::Cancelled => "cancelled",
        }
    }
}

impl UserTransaction {
    /// Find a transaction by ID
    pub fn find_by_id(id: i32, conn: &mut diesel::PgConnection) -> QueryResult<Self> {
        user_transactions::table.find(id).first(conn)
    }

    /// Find a transaction by hash
    pub fn find_by_hash(tx_hash: &str, conn: &mut diesel::PgConnection) -> QueryResult<Self> {
        user_transactions::table
            .filter(user_transactions::tx_hash.eq(tx_hash))
            .first(conn)
    }

    /// Check if a transaction exists by hash
    pub fn exists_by_hash(tx_hash: &str, conn: &mut diesel::PgConnection) -> QueryResult<bool> {
        select(exists(
            user_transactions::table.filter(user_transactions::tx_hash.eq(tx_hash)),
        ))
        .get_result(conn)
    }

    /// Find all transactions for a user
    pub fn find_by_user(
        user_address: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_transactions::table
            .filter(user_transactions::user_address.eq(user_address))
            .order(user_transactions::block_timestamp.desc())
            .load(conn)
    }

    /// Find all transactions for a vault
    pub fn find_by_vault(
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_transactions::table
            .filter(user_transactions::vault_id.eq(vault_id))
            .order(user_transactions::block_timestamp.desc())
            .load(conn)
    }

    /// Find transactions for a user in a specific vault
    pub fn find_by_user_and_vault(
        user_address: &str,
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_transactions::table
            .filter(user_transactions::user_address.eq(user_address))
            .filter(user_transactions::vault_id.eq(vault_id))
            .order(user_transactions::block_timestamp.desc())
            .load(conn)
    }

    /// Find transactions for a user in a specific vault ordered chronologically (for KPI calculations)
    pub fn find_by_user_and_vault_chronological(
        user_address: &str,
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_transactions::table
            .filter(user_transactions::user_address.eq(user_address))
            .filter(user_transactions::vault_id.eq(vault_id))
            .order(user_transactions::block_timestamp.asc())
            .load(conn)
    }

    /// Find transactions by type
    pub fn find_by_type(
        transaction_type: TransactionType,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_transactions::table
            .filter(user_transactions::type_.eq(transaction_type.as_str()))
            .order(user_transactions::block_timestamp.desc())
            .load(conn)
    }

    /// Find transactions by status
    pub fn find_by_status(
        status: TransactionStatus,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_transactions::table
            .filter(user_transactions::status.eq(status.as_str()))
            .order(user_transactions::block_timestamp.desc())
            .load(conn)
    }

    /// Find pending transactions
    pub fn find_pending(conn: &mut diesel::PgConnection) -> QueryResult<Vec<Self>> {
        Self::find_by_status(TransactionStatus::Pending, conn)
    }

    /// Find confirmed transactions
    pub fn find_confirmed(conn: &mut diesel::PgConnection) -> QueryResult<Vec<Self>> {
        Self::find_by_status(TransactionStatus::Confirmed, conn)
    }

    /// Find failed transactions
    pub fn find_failed(conn: &mut diesel::PgConnection) -> QueryResult<Vec<Self>> {
        Self::find_by_status(TransactionStatus::Failed, conn)
    }

    /// Find transactions in a block range
    pub fn find_by_block_range(
        start_block: i64,
        end_block: i64,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_transactions::table
            .filter(user_transactions::block_number.ge(start_block))
            .filter(user_transactions::block_number.le(end_block))
            .order(user_transactions::block_number.asc())
            .load(conn)
    }

    /// Find recent transactions (last N transactions)
    pub fn find_recent(limit: i64, conn: &mut diesel::PgConnection) -> QueryResult<Vec<Self>> {
        user_transactions::table
            .order(user_transactions::block_timestamp.desc())
            .limit(limit)
            .load(conn)
    }

    /// Create a new transaction
    pub fn create(
        new_transaction: &NewUserTransaction,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        diesel::insert_into(user_transactions::table)
            .values(new_transaction)
            .get_result(conn)
    }

    /// Update a transaction
    pub fn update(
        &self,
        updates: &UserTransactionUpdate,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        diesel::update(user_transactions::table.find(self.id))
            .set(updates)
            .get_result(conn)
    }

    /// Update transaction status
    pub fn update_status(
        &self,
        status: TransactionStatus,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        let updates = UserTransactionUpdate {
            status: Some(status.as_str().to_string()),
            shares_amount: None,
            share_price: None,
            gas_fee: None,
            metadata: None,
            updated_at: Some(Utc::now()),
        };
        self.update(&updates, conn)
    }

    /// Delete a transaction
    pub fn delete(id: i32, conn: &mut diesel::PgConnection) -> QueryResult<usize> {
        diesel::delete(user_transactions::table.find(id)).execute(conn)
    }

    /// Count total transactions
    pub fn count(conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        user_transactions::table.count().get_result(conn)
    }

    /// Count transactions by user
    pub fn count_by_user(user_address: &str, conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        user_transactions::table
            .filter(user_transactions::user_address.eq(user_address))
            .count()
            .get_result(conn)
    }

    /// Count transactions by vault
    pub fn count_by_vault(vault_id: &str, conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        user_transactions::table
            .filter(user_transactions::vault_id.eq(vault_id))
            .count()
            .get_result(conn)
    }

    /// Count transactions by type
    pub fn count_by_type(
        transaction_type: TransactionType,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<i64> {
        user_transactions::table
            .filter(user_transactions::type_.eq(transaction_type.as_str()))
            .count()
            .get_result(conn)
    }

    /// Count transactions by status
    pub fn count_by_status(
        status: TransactionStatus,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<i64> {
        user_transactions::table
            .filter(user_transactions::status.eq(status.as_str()))
            .count()
            .get_result(conn)
    }

    /// Calculate total volume for a vault
    pub fn total_volume_by_vault(
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Option<Decimal>> {
        use diesel::dsl::sum;

        user_transactions::table
            .filter(user_transactions::vault_id.eq(vault_id))
            .filter(user_transactions::status.eq(TransactionStatus::Confirmed.as_str()))
            .select(sum(user_transactions::amount))
            .first(conn)
    }

    /// Calculate total volume for a user
    pub fn total_volume_by_user(
        user_address: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Option<Decimal>> {
        use diesel::dsl::sum;

        user_transactions::table
            .filter(user_transactions::user_address.eq(user_address))
            .filter(user_transactions::status.eq(TransactionStatus::Confirmed.as_str()))
            .select(sum(user_transactions::amount))
            .first(conn)
    }

    /// Calculate total deposits for a user in a specific vault
    pub fn total_deposits_by_user_and_vault(
        user_address: &str,
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Option<Decimal>> {
        use diesel::dsl::sum;

        user_transactions::table
            .filter(user_transactions::user_address.eq(user_address))
            .filter(user_transactions::vault_id.eq(vault_id))
            .filter(user_transactions::type_.eq(TransactionType::Deposit.as_str()))
            .filter(user_transactions::status.eq(TransactionStatus::Confirmed.as_str()))
            .select(sum(user_transactions::amount))
            .first(conn)
    }

    /// Find transactions for a user in a specific vault with pagination
    /// Uses ID-based cursor pagination for efficient database queries
    pub fn find_by_user_and_vault_paginated(
        user_address: &str,
        vault_id: &str,
        transaction_type: Option<&str>,
        cursor_id: Option<i32>,
        limit: i64,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        use diesel::prelude::*;

        match (transaction_type, cursor_id) {
            (Some(tx_type), Some(cursor)) => {
                // Both filters applied
                user_transactions::table
                    .filter(user_transactions::user_address.eq(user_address))
                    .filter(user_transactions::vault_id.eq(vault_id))
                    .filter(user_transactions::type_.eq(tx_type))
                    .filter(user_transactions::id.lt(cursor))
                    .order(user_transactions::block_timestamp.desc())
                    .limit(limit)
                    .load(conn)
            }
            (Some(tx_type), None) => {
                // Only transaction type filter
                user_transactions::table
                    .filter(user_transactions::user_address.eq(user_address))
                    .filter(user_transactions::vault_id.eq(vault_id))
                    .filter(user_transactions::type_.eq(tx_type))
                    .order(user_transactions::block_timestamp.desc())
                    .limit(limit)
                    .load(conn)
            }
            (None, Some(cursor)) => {
                // Only cursor filter
                user_transactions::table
                    .filter(user_transactions::user_address.eq(user_address))
                    .filter(user_transactions::vault_id.eq(vault_id))
                    .filter(user_transactions::id.lt(cursor))
                    .order(user_transactions::block_timestamp.desc())
                    .limit(limit)
                    .load(conn)
            }
            (None, None) => {
                // No filters
                user_transactions::table
                    .filter(user_transactions::user_address.eq(user_address))
                    .filter(user_transactions::vault_id.eq(vault_id))
                    .order(user_transactions::block_timestamp.desc())
                    .limit(limit)
                    .load(conn)
            }
        }
    }

    /// Find pending redeem transaction by `redeem_id` from metadata
    pub fn find_pending_redeem_by_id(
        user_address: &str,
        vault_id: &str,
        redeem_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        use diesel::prelude::*;

        user_transactions::table
            .filter(user_transactions::user_address.eq(user_address))
            .filter(user_transactions::vault_id.eq(vault_id))
            .filter(user_transactions::type_.eq(TransactionType::Withdraw.as_str()))
            .filter(user_transactions::status.eq(TransactionStatus::Pending.as_str()))
            .filter(user_transactions::metadata.is_not_null())
            .load::<Self>(conn)?
            .into_iter()
            .find(|tx| {
                if let Some(metadata) = &tx.metadata
                    && let Some(id) = metadata.get("redeem_id")
                    && let Some(id_str) = id.as_str()
                {
                    return id_str == redeem_id;
                }

                false
            })
            .ok_or(diesel::result::Error::NotFound)
    }

    /// Update transaction status and amount with new transaction hash
    pub fn update_status_and_amount(
        id: i32,
        status: &str,
        amount: Decimal,
        tx_hash: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        use diesel::prelude::*;

        diesel::update(user_transactions::table.find(id))
            .set((
                user_transactions::status.eq(status),
                user_transactions::amount.eq(amount),
                user_transactions::tx_hash.eq(tx_hash),
                user_transactions::updated_at.eq(Utc::now()),
            ))
            .get_result(conn)
    }
}
