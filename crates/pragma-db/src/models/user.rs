use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use pragma_common::web3::Chain;

use crate::schema::users;

#[derive(Debug, Clone, Serialize, Deserialize, Queryable, Selectable, Identifiable)]
#[diesel(table_name = users)]
#[diesel(primary_key(address))]
#[diesel(check_for_backend(diesel::pg::Pg))]
pub struct User {
    pub address: String,
    pub chain: String,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

#[derive(Debug, Clone, Serialize, Deserialize, Insertable)]
#[diesel(table_name = users)]
pub struct NewUser {
    pub address: String,
    pub chain: String,
}

impl User {
    pub fn find_by_address(address: &str, conn: &mut diesel::PgConnection) -> QueryResult<Self> {
        users::table.find(address).first(conn)
    }

    pub fn find_all(conn: &mut diesel::PgConnection) -> QueryResult<Vec<Self>> {
        users::table.load(conn)
    }

    pub fn find_by_chain(chain: &str, conn: &mut diesel::PgConnection) -> QueryResult<Vec<Self>> {
        users::table.filter(users::chain.eq(chain)).load(conn)
    }

    pub fn create(new_user: &NewUser, conn: &mut diesel::PgConnection) -> QueryResult<Self> {
        diesel::insert_into(users::table)
            .values(new_user)
            .get_result(conn)
    }

    pub fn find_or_create(
        address: &str,
        chain: Chain,
        conn: &mut diesel::PgConnection,
    ) -> QueryResult<Self> {
        match Self::find_by_address(address, conn) {
            Ok(user) => Ok(user),
            Err(diesel::NotFound) => {
                let new_user = NewUser {
                    address: address.to_string(),
                    chain: chain.to_string(),
                };
                Self::create(&new_user, conn)
            }
            Err(e) => Err(e),
        }
    }

    pub fn count(conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        users::table.count().get_result(conn)
    }

    pub fn count_by_chain(chain: &str, conn: &mut diesel::PgConnection) -> QueryResult<i64> {
        users::table
            .filter(users::chain.eq(chain))
            .count()
            .get_result(conn)
    }
}
