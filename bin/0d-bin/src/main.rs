mod cli;

use crate::cli::AuthCli;
use anyhow::Result;
use clap::Parser;
use dotenvy::dotenv;
use pragma_common::{services::ServiceGroup, telemetry::init_telemetry};

use pragma_common::services::Service;
use pragma_common::starknet::FallbackProvider;
use url::Url;
use zerod_api::{ApiService, AppState};
use zerod_db::{init_pool, run_migrations};
use zerod_indexer::task::IndexerTask;
use zerod_kpi::KpiTask;

/// The list of all the starknet rpcs that the FallbackProvider may use.
/// They're sorted by priority (so we sorted them by reliability here).
pub const STARKNET_RPC_URLS: [&str; 2] = [
    "https://api.cartridge.gg/x/starknet/mainnet",
    "https://rpc.pathfinder.equilibrium.co/mainnet/rpc/v0_9",
];

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();

    let AuthCli {
        otel_collector_endpoint,
        database_url,
        api_port,
        apibara_api_key,
    } = AuthCli::parse();

    let app_name = "0d_master_api";
    if let Err(e) = init_telemetry(app_name, otel_collector_endpoint) {
        panic!("Could not init telemetry: {e}");
    }

    let starknet_provider = FallbackProvider::new(
        STARKNET_RPC_URLS
            .iter()
            .map(|url| Url::parse(url).expect("Invalid Starknet RPC url"))
            .collect(),
    )
    .expect("Could not init the starknet provider");

    let pool = init_pool(app_name, &database_url)?;
    run_migrations(&pool).await?;

    let app_state = AppState { pool: pool.clone() };

    let api_service = ApiService::new(app_state, "0.0.0.0", api_port);

    let indexer_service =
        IndexerTask::new(pool.clone(), apibara_api_key, starknet_provider.clone());

    let kpi_service = KpiTask::new(pool.clone());

    ServiceGroup::default()
        .with(api_service)
        .with(indexer_service)
        .with(kpi_service)
        .start_and_drive_to_end()
        .await?;

    Ok(())
}
