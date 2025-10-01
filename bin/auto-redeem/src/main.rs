use std::sync::Arc;

use anyhow::{Context, Result, bail};
use clap::Parser;
use dotenvy::dotenv;
use pragma_common::services::Service;
use pragma_common::starknet::FallbackProvider;
use tracing_subscriber::EnvFilter;
use url::Url;

use zero_d_relayer::{EvianStarknetClient, RelayerConfig, RelayerTask, StarknetAccountConfig};

#[derive(Parser, Debug)]
#[command(author, version, about = "0D Finance auto-redeem runner", long_about = None)]
struct Cli {
    #[arg(long, env = "DATABASE_URL")]
    database_url: String,

    #[arg(long, env = "STARKNET_ACCOUNT_ADDRESS")]
    starknet_account_address: String,

    #[arg(long, env = "STARKNET_PRIVATE_KEY")]
    starknet_private_key: String,

    #[arg(
        long = "starknet-rpc",
        env = "STARKNET_RPC_URLS",
        value_delimiter = ',',
        value_name = "URL",
        help = "Comma-separated list of Starknet RPC endpoints"
    )]
    starknet_rpc_urls: Vec<String>,
}

fn init_logger() {
    let env_filter = EnvFilter::try_from_default_env().unwrap_or_else(|_| EnvFilter::new("info"));
    tracing_subscriber::fmt().with_env_filter(env_filter).init();
}

#[tokio::main]
async fn main() -> Result<()> {
    dotenv().ok();
    init_logger();

    let cli = Cli::parse();

    let rpc_urls = cli
        .starknet_rpc_urls
        .iter()
        .map(|url| Url::parse(url).context("Invalid Starknet RPC URL"))
        .collect::<Result<Vec<_>>>()?;

    if rpc_urls.is_empty() {
        bail!("At least one Starknet RPC URL must be provided");
    }

    let provider = FallbackProvider::new(rpc_urls).context("Failed to create Starknet provider")?;

    let account_address = starknet::core::types::Felt::from_hex(&cli.starknet_account_address)
        .context("Invalid Starknet account address")?;
    let private_key = starknet_crypto::Felt::from_hex(&cli.starknet_private_key)
        .context("Invalid Starknet private key")?;
    let account_config = StarknetAccountConfig::new(account_address, private_key);

    let starknet_client = Arc::new(EvianStarknetClient::new(provider, &account_config));

    let pool = pragma_db::init_pool("auto_redeem", &cli.database_url)
        .context("Failed to initialise database pool")?;

    let relayer_config = RelayerConfig::default();
    let task = RelayerTask::new(pool, starknet_client, relayer_config);

    task.start_and_drive_to_end().await?;

    Ok(())
}
