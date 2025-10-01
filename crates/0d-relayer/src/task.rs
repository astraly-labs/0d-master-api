use std::sync::Arc;

use anyhow::Result;
use deadpool_diesel::postgres::Pool;
use pragma_common::services::{Service, ServiceRunner};

use crate::config::RelayerConfig;
use crate::repository::RelayerRepository;
use crate::service::RelayerService;
use crate::starknet::StarknetClient;

pub struct RelayerTask {
    repository: RelayerRepository,
    starknet: Arc<dyn StarknetClient>,
    config: RelayerConfig,
}

impl RelayerTask {
    pub fn new(pool: Pool, starknet: Arc<dyn StarknetClient>, config: RelayerConfig) -> Self {
        Self {
            repository: RelayerRepository::new(pool),
            starknet,
            config,
        }
    }
}

#[async_trait::async_trait]
impl Service for RelayerTask {
    async fn start<'a>(&mut self, mut runner: ServiceRunner<'a>) -> Result<()> {
        let repository = self.repository.clone();
        let starknet = self.starknet.clone();
        let config = self.config.clone();

        runner.spawn_loop(move |ctx| async move {
            let service = RelayerService::new(repository, starknet, config);
            if let Some(result) = ctx
                .run_until_cancelled(service.run_forever(ctx.token.clone()))
                .await
            {
                result?;
            }
            Ok::<(), anyhow::Error>(())
        });

        Ok(())
    }
}
