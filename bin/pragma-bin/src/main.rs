mod cli;

use crate::cli::AuthCli;
use anyhow::Result;
use clap::Parser;
use dotenvy::dotenv;
use pragma_common::{services::ServiceGroup, telemetry::init_telemetry};

use pragma_api::{ApiService, AppState};
use pragma_common::services::Service;
use pragma_db::{init_pool, run_migrations};
use pragma_indexer::task::IndexerTask;

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let AuthCli {
        otel_collector_endpoint,
        database_url,
        api_port,
    } = AuthCli::parse();

    let app_name = "0d_master_api";
    if let Err(e) = init_telemetry(app_name, otel_collector_endpoint) {
        panic!("Could not init telemetry: {e}");
    }

    let pool = init_pool(&app_name, &database_url)?;
    run_migrations(&pool).await;

    let app_state = AppState { pool: pool.clone() };

    let api_service = ApiService::new(app_state, "0.0.0.0", api_port);

    let indexer_service = IndexerTask::new(pool.clone());

    ServiceGroup::default()
        .with(api_service)
        .with(indexer_service)
        .start_and_drive_to_end()
        .await?;

    Ok(())
}
