use bigdecimal::BigDecimal;
use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::user_positions;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable)]
#[diesel(table_name = user_positions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserPosition {
    pub id: i32,
    pub user_address: String,
    pub vault_id: String,
    pub share_balance: BigDecimal,
    pub cost_basis: BigDecimal,
    pub first_deposit_at: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = user_positions)]
pub struct NewUserPosition {
    pub user_address: String,
    pub vault_id: String,
    pub share_balance: BigDecimal,
    pub cost_basis: BigDecimal,
    pub first_deposit_at: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, AsChangeset)]
#[diesel(table_name = user_positions)]
pub struct UserPositionUpdate {
    pub share_balance: Option<BigDecimal>,
    pub cost_basis: Option<BigDecimal>,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl UserPosition {
    /// Find a position by ID
    pub fn find_by_id(id: i32, conn: &mut diesel::PgConnection) -> QueryResult<Self> {
        user_positions::table.find(id).first(conn)
    }

    /// Find all positions for a user
    pub fn find_by_user(
        user_address: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_positions::table
            .filter(user_positions::user_address.eq(user_address))
            .load(conn)
    }

    /// Find all positions for a vault
    pub fn find_by_vault(
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_positions::table
            .filter(user_positions::vault_id.eq(vault_id))
            .load(conn)
    }

    /// Find a specific user's position in a vault
    pub fn find_by_user_and_vault(
        user_address: &str,
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        user_positions::table
            .filter(user_positions::user_address.eq(user_address))
            .filter(user_positions::vault_id.eq(vault_id))
            .first(conn)
    }

    /// Find all active positions (`share_balance` > 0)
    pub fn find_active(conn: &mut diesel::PgConnection) -> QueryResult<Vec<Self>> {
        user_positions::table
            .filter(user_positions::share_balance.gt(BigDecimal::from(0)))
            .load(conn)
    }

    /// Find active positions for a user
    pub fn find_active_by_user(
        user_address: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_positions::table
            .filter(user_positions::user_address.eq(user_address))
            .filter(user_positions::share_balance.gt(BigDecimal::from(0)))
            .load(conn)
    }

    /// Find active positions for a vault
    pub fn find_active_by_vault(
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_positions::table
            .filter(user_positions::vault_id.eq(vault_id))
            .filter(user_positions::share_balance.gt(BigDecimal::from(0)))
            .load(conn)
    }

    /// Create a new position
    pub fn create(
        new_position: &NewUserPosition,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        diesel::insert_into(user_positions::table)
            .values(new_position)
            .get_result(conn)
    }

    /// Update a position
    pub fn update(
        &self,
        updates: &UserPositionUpdate,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        diesel::update(user_positions::table.find(self.id))
            .set(updates)
            .get_result(conn)
    }

    /// Delete a position
    pub fn delete(id: i32, conn: &mut diesel::PgConnection) -> QueryResult<usize> {
        diesel::delete(user_positions::table.find(id)).execute(conn)
    }

    /// Count total positions
    pub fn count(conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        user_positions::table.count().get_result(conn)
    }

    /// Count active positions
    pub fn count_active(conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        user_positions::table
            .filter(user_positions::share_balance.gt(BigDecimal::from(0)))
            .count()
            .get_result(conn)
    }

    /// Count positions for a vault
    pub fn count_by_vault(vault_id: &str, conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        user_positions::table
            .filter(user_positions::vault_id.eq(vault_id))
            .count()
            .get_result(conn)
    }

    /// Count active positions for a vault
    pub fn count_active_by_vault(
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<i64> {
        user_positions::table
            .filter(user_positions::vault_id.eq(vault_id))
            .filter(user_positions::share_balance.gt(BigDecimal::from(0)))
            .count()
            .get_result(conn)
    }
}
