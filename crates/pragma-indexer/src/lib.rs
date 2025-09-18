pub mod task;
pub mod vaults;

use deadpool_diesel::postgres::Pool;
use starknet::core::types::Felt;
use std::time::Duration;
use task_supervisor::SupervisorBuilder;

use crate::vaults::extended::ExtendedVault;

pub struct IndexerService {
    db_pool: Pool,
}

impl IndexerService {
    pub const fn new(db_pool: Pool) -> Self {
        Self { db_pool }
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
                    db_pool: self.db_pool.clone(),
                    current_block: 0,
                    current_timestamp: None,
                    apibara_api_key: String::new(),
                    vault_address: Felt::from_hex("0x0").expect("Invalid vault address"),
                },
            );

        let supervisor_handle = supervisor.build().run();

        supervisor_handle.wait().await?;
        anyhow::bail!("Indexer Supervisor stopped! 😨");
    }
}
