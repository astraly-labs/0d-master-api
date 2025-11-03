pub mod task;
pub mod vaults;

use deadpool_diesel::postgres::Pool;
use pragma_common::starknet::FallbackProvider;
use starknet::core::types::Felt;
use std::time::Duration;
use task_supervisor::SupervisorBuilder;
use zerod_db::ZerodPool;
use zerod_db::models::Vault;

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
            .with_dead_tasks_threshold(Some(0.5)) // if any task is dead, stop the supervisor
            .with_base_restart_delay(Duration::from_millis(500))
            .with_max_restart_attempts(5)
            .with_task_being_stable_after(Duration::from_secs(120))
            .with_health_check_interval(Duration::from_secs(5));

        let vaults = self
            .db_pool
            .interact_with_context("fetch all vaults for indexer".to_string(), Vault::find_all)
            .await
            .map_err(|e| anyhow::anyhow!("Database interaction error: {e}"))?;

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
                    vault_address: Felt::from_hex(&vault.contract_address).unwrap_or_else(|_| {
                        panic!("Invalid vault address: {}", vault.contract_address)
                    }),
                    proxy_address: vault.proxy_address.and_then(|v| Felt::from_hex(&v).ok()),
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
