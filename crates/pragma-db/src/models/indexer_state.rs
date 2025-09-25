use chrono::{DateTime, Utc};
use diesel::prelude::*;
use serde::{Deserialize, Serialize};

use crate::schema::indexer_state;

/// Indexer status variants
#[derive(Debug, Clone, PartialEq, Eq, Serialize, Deserialize)]
pub enum IndexerStatus {
    Active,
    Paused,
    Error,
    Synced,
}

impl IndexerStatus {
    /// Convert to string for database storage
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Active => "active",
            Self::Paused => "paused",
            Self::Error => "error",
            Self::Synced => "synced",
        }
    }
}

impl std::fmt::Display for IndexerStatus {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "{}", self.as_str())
    }
}

impl From<IndexerStatus> for String {
    fn from(status: IndexerStatus) -> Self {
        status.as_str().to_string()
    }
}

impl From<String> for IndexerStatus {
    fn from(s: String) -> Self {
        match s.as_str() {
            "active" => Self::Active,
            "paused" => Self::Paused,
            "error" => Self::Error,
            "synced" => Self::Synced,
            _ => unreachable!("Invalid indexer status: {s}"),
        }
    }
}

impl From<&str> for IndexerStatus {
    fn from(s: &str) -> Self {
        match s {
            "active" => Self::Active,
            "paused" => Self::Paused,
            "error" => Self::Error,
            "synced" => Self::Synced,
            _ => unreachable!("Invalid indexer status: {s}"),
        }
    }
}

#[derive(Queryable, Selectable, Identifiable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = indexer_state)]
pub struct IndexerState {
    pub id: i32,
    pub vault_id: String,
    pub last_processed_block: i64,
    pub last_processed_timestamp: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub last_error_at: Option<DateTime<Utc>>,
    pub status: Option<String>,
    pub created_at: Option<DateTime<Utc>>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl IndexerState {
    /// Get the status as an enum
    pub fn status_enum(&self) -> Option<IndexerStatus> {
        self.status
            .as_ref()
            .map(|s| IndexerStatus::from(s.as_str()))
    }

    /// Set the status from an enum
    pub fn set_status_enum(&mut self, status: IndexerStatus) {
        self.status = Some(status.as_str().to_string());
    }

    /// Check if the indexer is in error state
    pub fn is_error(&self) -> bool {
        self.status_enum() == Some(IndexerStatus::Error)
    }

    /// Check if the indexer is running
    pub fn is_running(&self) -> bool {
        self.status_enum() == Some(IndexerStatus::Active)
    }

    /// Check if the indexer is synced
    pub fn is_synced(&self) -> bool {
        self.status_enum() == Some(IndexerStatus::Synced)
    }
}

#[derive(Insertable, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = indexer_state)]
pub struct NewIndexerState {
    pub vault_id: String,
    pub last_processed_block: i64,
    pub last_processed_timestamp: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub last_error_at: Option<DateTime<Utc>>,
    pub status: Option<String>,
}

#[derive(Default, AsChangeset, Serialize, Deserialize, Debug, Clone)]
#[diesel(table_name = indexer_state)]
pub struct IndexerStateUpdate {
    pub last_processed_block: Option<i64>,
    pub last_processed_timestamp: Option<DateTime<Utc>>,
    pub last_error: Option<String>,
    pub last_error_at: Option<DateTime<Utc>>,
    pub status: Option<String>,
    pub updated_at: Option<DateTime<Utc>>,
}

impl IndexerState {
    /// Find all indexer states
    pub fn find_all(conn: &mut PgConnection) -> QueryResult<Vec<Self>> {
        indexer_state::table.load::<Self>(conn)
    }

    /// Find indexer state by `vault_id`
    pub fn find_by_vault_id(
        vault_id: &str,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        indexer_state::table
            .filter(indexer_state::vault_id.eq(vault_id))
            .first(conn)
    }

    /// Create a new indexer state
    pub fn create(
        new_state: &NewIndexerState,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        diesel::insert_into(indexer_state::table)
            .values(new_state)
            .returning(Self::as_returning())
            .get_result(conn)
    }

    /// Update the indexer state
    pub fn update(
        &self,
        updates: &IndexerStateUpdate,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        diesel::update(indexer_state::table.filter(indexer_state::id.eq(self.id)))
            .set(updates)
            .returning(Self::as_returning())
            .get_result(conn)
    }

    /// Update or create indexer state for a vault
    pub fn upsert_for_vault(
        vault_id: &str,
        last_processed_block: i64,
        last_processed_timestamp: Option<DateTime<Utc>>,
        status: Option<IndexerStatus>,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        let status_string = status.map(|s| s.as_str().to_string());

        match Self::find_by_vault_id(vault_id, conn) {
            Ok(state) => {
                // Update existing state
                let updates = IndexerStateUpdate {
                    last_processed_block: Some(last_processed_block),
                    last_processed_timestamp,
                    status: status_string,
                    last_error: None, // Clear any previous errors
                    last_error_at: None,
                    updated_at: Some(Utc::now()),
                };
                state.update(&updates, conn)
            }
            Err(diesel::result::Error::NotFound) => {
                // Create new state
                let new_state = NewIndexerState {
                    vault_id: vault_id.to_string(),
                    last_processed_block,
                    last_processed_timestamp,
                    last_error: None,
                    last_error_at: None,
                    status: status_string,
                };
                Self::create(&new_state, conn)
            }
            Err(e) => Err(e),
        }
    }

    /// Update indexer state with status preservation
    pub fn update_with_status_preservation(
        vault_id: &str,
        last_processed_block: i64,
        last_processed_timestamp: Option<DateTime<Utc>>,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        // Get current state to check status
        let current_state = Self::find_by_vault_id(vault_id, conn)?;

        // Preserve synced status, otherwise set to active
        let new_status = if current_state.is_synced() {
            IndexerStatus::Synced
        } else {
            IndexerStatus::Active
        };

        // Update with preserved status
        let updates = IndexerStateUpdate {
            last_processed_block: Some(last_processed_block),
            last_processed_timestamp,
            status: Some(new_status.as_str().to_string()),
            last_error: None, // Clear any previous errors
            last_error_at: None,
            updated_at: Some(Utc::now()),
        };

        current_state.update(&updates, conn)
    }

    /// Record an error for the indexer state
    pub fn record_error(
        &self,
        error_message: String,
        conn: &mut PgConnection,
    ) -> Result<Self, diesel::result::Error> {
        let updates = IndexerStateUpdate {
            last_error: Some(error_message),
            last_error_at: Some(Utc::now()),
            status: Some(IndexerStatus::Error.as_str().to_string()),
            updated_at: Some(Utc::now()),
            ..Default::default()
        };
        self.update(&updates, conn)
    }
}
