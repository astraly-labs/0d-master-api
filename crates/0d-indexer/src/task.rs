use deadpool_diesel::postgres::Pool;
use pragma_common::services::{Service, ServiceRunner};
use pragma_common::starknet::FallbackProvider;

use crate::IndexerService;

pub struct IndexerTask {
    db_pool: Pool,
    apibara_api_key: String,
    starknet_provider: FallbackProvider,
}

impl IndexerTask {
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
}

#[async_trait::async_trait]
impl Service for IndexerTask {
    async fn start<'a>(&mut self, mut runner: ServiceRunner<'a>) -> anyhow::Result<()> {
        let db_pool = self.db_pool.clone();
        let apibara_api_key = self.apibara_api_key.clone();
        let starknet_provider = self.starknet_provider.clone();

        runner.spawn_loop(move |ctx| async move {
            let indexer_service =
                IndexerService::new(db_pool.clone(), apibara_api_key, starknet_provider.clone());
            if let Some(result) = ctx.run_until_cancelled(indexer_service.run_forever()).await {
                result?;
            }

            anyhow::Ok(())
        });

        Ok(())
    }
}
