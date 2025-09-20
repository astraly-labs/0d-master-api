pub mod task;
pub mod vaults;

use deadpool_diesel::postgres::Pool;
use starknet::core::types::Felt;
use std::time::Duration;
use task_supervisor::SupervisorBuilder;

use crate::vaults::{extended::ExtendedVault, state::VaultState};

pub struct IndexerService {
    db_pool: Pool,
    extended_vault_adress: String,
    extended_vault_start_block: u64,
    apibara_api_key: String,
}

impl IndexerService {
    pub const fn new(
        db_pool: Pool,
        extended_vault_adress: String,
        extended_vault_start_block: u64,
        apibara_api_key: String,
    ) -> Self {
        Self {
            db_pool,
            extended_vault_adress,
            extended_vault_start_block,
            apibara_api_key,
        }
    }

    pub async fn run_forever(&self) -> anyhow::Result<()> {
        let supervisor = SupervisorBuilder::default()
            .with_dead_tasks_threshold(Some(0.00)) // if any task is dead, stop the supervisor
            .with_base_restart_delay(Duration::from_millis(500))
            .with_max_restart_attempts(5)
            .with_task_being_stable_after(Duration::from_secs(120))
            .with_health_check_interval(Duration::from_secs(5))
            // VAULTS TASKS
            .with_task(
                ExtendedVault::vault_id(),
                ExtendedVault {
                    vault_address: Felt::from_hex(&self.extended_vault_adress)
                        .expect("Invalid vault address"),
                    apibara_api_key: self.apibara_api_key.clone(),
                    state: VaultState::new(self.extended_vault_start_block, self.db_pool.clone()),
                },
            );

        let supervisor_handle = supervisor.build().run();

        supervisor_handle.wait().await?;
        anyhow::bail!("Indexer Supervisor stopped! 😨");
    }
}
