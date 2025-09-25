pub mod task;
pub mod vaults;

use deadpool_diesel::postgres::Pool;
use pragma_common::starknet::FallbackProvider;
use pragma_db::models::Vault;
use starknet::core::types::Felt;
use std::time::Duration;
use task_supervisor::SupervisorBuilder;

use crate::vaults::{starknet::StarknetIndexer, state::VaultState};

pub struct IndexerService {
    db_pool: Pool,
    apibara_api_key: String,
    starknet_provider: FallbackProvider,
}

impl IndexerService {
    pub const fn new(
        db_pool: Pool,
        apibara_api_key: String,
        starknet_provider: FallbackProvider,
    ) -> Self {
        Self {
            db_pool,
            apibara_api_key,
            starknet_provider,
        }
    }

    pub async fn run_forever(&self) -> anyhow::Result<()> {
        let mut supervisor = SupervisorBuilder::default()
            .with_dead_tasks_threshold(Some(0.00)) // if any task is dead, stop the supervisor
            .with_base_restart_delay(Duration::from_millis(500))
            .with_max_restart_attempts(5)
            .with_task_being_stable_after(Duration::from_secs(120))
            .with_health_check_interval(Duration::from_secs(5));

        let conn = self
            .db_pool
            .get()
            .await
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {e}"))
            .map_err(|e| anyhow::anyhow!("Failed to get database connection: {e}"))?;

        let vaults = conn
            .interact(Vault::find_all)
            .await
            .map_err(|e| anyhow::anyhow!("Database interaction error: {e}"))??;

        if vaults.is_empty() {
            anyhow::bail!("No vaults found in the database!");
        }

        for vault in vaults {
            tracing::info!(
                "Starting indexer for vault: {} at block {}",
                vault.id,
                vault.start_block
            );
            supervisor = supervisor.with_task(
                &vault.id,
                StarknetIndexer {
                    vault_id: vault.id.clone(),
                    vault_address: Felt::from_hex(&vault.contract_address).unwrap(),
                    apibara_api_key: self.apibara_api_key.clone(),
                    starknet_provider: self.starknet_provider.clone(),
                    state: VaultState::new(
                        vault.id.clone(),
                        vault.start_block as u64,
                        self.db_pool.clone(),
                    ),
                },
            );
        }

        let supervisor_handle = supervisor.build().run();

        supervisor_handle.wait().await?;
        anyhow::bail!("Indexer Supervisor stopped! 😨");
    }
}
