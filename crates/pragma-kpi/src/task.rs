use deadpool_diesel::postgres::Pool;
use pragma_common::services::{Service, ServiceRunner};

use crate::service::KpiService;

pub struct KpiTask {
    db_pool: Pool,
}

impl KpiTask {
    pub const fn new(db_pool: Pool) -> Self {
        Self { db_pool }
    }
}

#[async_trait::async_trait]
impl Service for KpiTask {
    async fn start<'a>(&mut self, mut runner: ServiceRunner<'a>) -> anyhow::Result<()> {
        let db_pool = self.db_pool.clone();

        runner.spawn_loop(move |ctx| async move {
            let kpi_service = KpiService::new(db_pool.clone());

            if let Some(result) = ctx.run_until_cancelled(kpi_service.run_forever()).await {
                result?;
            }

            anyhow::Ok(())
        });

        Ok(())
    }
}
