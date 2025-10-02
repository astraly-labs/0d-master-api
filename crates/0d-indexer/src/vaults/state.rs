use chrono::{DateTime, Utc};
use deadpool_diesel::postgres::Pool;
use zerod_db::ZerodPool;
use zerod_db::models::indexer_state::{IndexerState, IndexerStateUpdate, IndexerStatus};

#[derive(Clone)]
pub struct VaultState {
    pub vault_id: String,
    pub current_block: u64,
    pub current_timestamp: Option<DateTime<Utc>>,
    pub db_pool: Pool,
}

impl VaultState {
    pub const fn new(vault_id: String, current_block: u64, db_pool: Pool) -> Self {
        Self {
            vault_id,
            current_block,
            current_timestamp: None,
            db_pool,
        }
    }

    /// Load the last processed block from the database
    pub async fn load_last_processed_block(&mut self, vault_id: &str) -> Result<(), anyhow::Error> {
        let vault_id = vault_id.to_string();

        match self
            .db_pool
            .interact_with_context(
                format!("load last processed block for vault: {}", self.vault_id),
                move |conn| IndexerState::find_by_vault_id(&vault_id, conn),
            )
            .await
        {
            Ok(state) => {
                if state.is_error() {
                    // If there was an error previously, start from the same block to retry
                    self.current_block = state.last_processed_block as u64;
                    tracing::warn!(
                        "[VaultState({})] ⚠️ Previous error detected, retrying from block {} (last error: {})",
                        self.vault_id,
                        self.current_block,
                        state.last_error.as_deref().unwrap_or("unknown error")
                    );
                } else {
                    // Resume from the last processed block + 1
                    self.current_block = (state.last_processed_block + 1) as u64;
                    tracing::info!(
                        "[VaultState({})] 📍 Resuming from block {} (last processed: {})",
                        self.vault_id,
                        self.current_block,
                        state.last_processed_block
                    );
                }
            }
            Err(zerod_db::DatabaseError::NotFound { .. }) => {
                // No previous state found, start from the configured block
                tracing::info!(
                    "[VaultState({})] 🆕 No previous state found, starting from block {}",
                    self.vault_id,
                    self.current_block
                );
            }
            Err(e) => {
                return Err(anyhow::anyhow!(
                    "[VaultState({})] 🗃️ Database interaction failed: {e}",
                    self.vault_id
                ));
            }
        }

        Ok(())
    }

    /// Initialize indexer state with the starting block
    pub async fn initialize_indexer_state(&self, vault_id: &str) -> Result<(), anyhow::Error> {
        let current_block = self.current_block;
        let vault_id = vault_id.to_string();

        self.db_pool
            .interact_with_context(
                format!("initialize indexer state for vault: {}", self.vault_id),
                move |conn| {
                    IndexerState::upsert_for_vault(
                        &vault_id,
                        current_block
                            .try_into()
                            .expect("[VaultState] 🌯 Block number too large for i64"),
                        None, // No timestamp for initialization
                        Some(IndexerStatus::Active),
                        conn,
                    )
                },
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!("[VaultState] 🗃️ Indexer state initialization failed: {e}")
            })?;

        Ok(())
    }

    /// Update indexer state with the last processed block
    pub async fn update_indexer_state(
        &self,
        vault_id: &str,
        block_number: u64,
        block_timestamp: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        let vault_id = vault_id.to_string();

        self.db_pool
            .interact_with_context(
                format!("update indexer state for vault: {}", self.vault_id),
                move |conn| {
                    IndexerState::update_with_status_preservation(
                        &vault_id,
                        block_number
                            .try_into()
                            .expect("[VaultState] 🌯 Block number too large for i64"),
                        Some(block_timestamp),
                        conn,
                    )
                },
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "[VaultState({})] 🗃️ Indexer state update failed: {e}",
                    self.vault_id
                )
            })?;

        Ok(())
    }

    /// Set indexer state to synced
    pub async fn set_indexer_state_synced(&self, vault_id: &str) -> Result<(), anyhow::Error> {
        let vault_id = vault_id.to_string();

        self.db_pool
            .interact_with_context(
                format!("set indexer state to synced for vault: {}", self.vault_id),
                move |conn| match IndexerState::find_by_vault_id(&vault_id, conn) {
                    Ok(state) => state.update(
                        &IndexerStateUpdate {
                            status: Some(IndexerStatus::Synced.as_str().to_string()),
                            updated_at: Some(Utc::now()),
                            ..Default::default()
                        },
                        conn,
                    ),
                    Err(e) => Err(e),
                },
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "[VaultState({})] 🗃️ Setting synced status failed: {e}",
                    self.vault_id
                )
            })?;

        Ok(())
    }

    /// Record an error in the indexer state
    pub async fn record_indexer_state_error(
        &self,
        vault_id: &str,
        error_message: String,
    ) -> Result<(), anyhow::Error> {
        let vault_id = vault_id.to_string();

        self.db_pool
            .interact_with_context(
                format!("record indexer error for vault: {}", self.vault_id),
                move |conn| match IndexerState::find_by_vault_id(&vault_id, conn) {
                    Ok(state) => state.record_error(error_message, conn),
                    Err(e) => Err(e),
                },
            )
            .await
            .map_err(|e| {
                anyhow::anyhow!(
                    "[VaultState({})] 🗃️ Error recording failed: {e}",
                    self.vault_id
                )
            })?;

        Ok(())
    }
}
