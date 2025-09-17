use deadpool_diesel::postgres::Pool;
use pragma_common::services::{Service, ServiceRunner};

use crate::IndexerService;

pub struct IndexerTask {
    db_pool: Pool,
}

impl IndexerTask {
    pub fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }
}

#[async_trait::async_trait]
impl Service for IndexerTask {
    async fn start<'a>(&mut self, mut runner: ServiceRunner<'a>) -> anyhow::Result<()> {
        let db_pool = self.db_pool.clone();

        runner.spawn_loop(move |ctx| async move {
            let indexer_service = IndexerService::new(db_pool.clone());
            if let Some(result) = ctx.run_until_cancelled(indexer_service.run_forever()).await {
                result?;
            }

            anyhow::Ok(())
        });

        Ok(())
    }
}
