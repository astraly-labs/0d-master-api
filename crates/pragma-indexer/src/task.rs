use deadpool_diesel::postgres::Pool;
use pragma_common::services::{Service, ServiceRunner};

use crate::IndexerService;

pub struct IndexerTask {
    db_pool: Pool,
    extended_vault_adress: String,
    extended_vault_start_block: u64,
    apibara_api_key: String,
}

impl IndexerTask {
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
}

#[async_trait::async_trait]
impl Service for IndexerTask {
    async fn start<'a>(&mut self, mut runner: ServiceRunner<'a>) -> anyhow::Result<()> {
        let db_pool = self.db_pool.clone();
        let extended_vault_adress = self.extended_vault_adress.clone();
        let extended_vault_start_block = self.extended_vault_start_block;
        let apibara_api_key = self.apibara_api_key.clone();

        runner.spawn_loop(move |ctx| async move {
            let indexer_service = IndexerService::new(
                db_pool.clone(),
                extended_vault_adress,
                extended_vault_start_block,
                apibara_api_key,
            );
            if let Some(result) = ctx.run_until_cancelled(indexer_service.run_forever()).await {
                result?;
            }

            anyhow::Ok(())
        });

        Ok(())
    }
}
