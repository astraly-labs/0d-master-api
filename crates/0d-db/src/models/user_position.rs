use chrono::{DateTime, Utc};
use diesel::prelude::*;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};

use crate::schema::user_positions;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable)]
#[diesel(table_name = user_positions)]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct UserPosition {
    pub id: i32,
    pub user_address: String,
    pub vault_id: String,
    pub share_balance: Decimal,
    pub cost_basis: Decimal,
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
    pub share_balance: Decimal,
    pub cost_basis: Decimal,
    pub first_deposit_at: Option<DateTime<Utc>>,
    pub last_activity_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, AsChangeset)]
#[diesel(table_name = user_positions)]
pub struct UserPositionUpdate {
    pub share_balance: Option<Decimal>,
    pub cost_basis: Option<Decimal>,
    pub last_activity_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl UserPosition {
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

    /// Find active positions for a vault
    pub fn find_active_by_vault(
        vault_id: &str,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Vec<Self>> {
        user_positions::table
            .filter(user_positions::vault_id.eq(vault_id))
            .filter(user_positions::share_balance.gt(Decimal::from(0)))
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
}
