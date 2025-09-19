use clap::Parser;

#[derive(Parser, Debug)]
#[command(author, version, about, long_about = None)]
pub struct AuthCli {
    /// Database URL
    #[arg(long, env = "DATABASE_URL")]
    pub database_url: String,

    /// OTEL collector endpoint
    #[arg(long, env = "OTEL_COLLECTOR_ENDPOINT")]
    pub otel_collector_endpoint: Option<String>,

    /// API port
    #[arg(long, env = "API_PORT", default_value = "8080")]
    pub api_port: u16,

    /// Starting block number for indexer
    #[arg(long, env = "EXTENDED_VAULT_START_BLOCK")]
    pub extended_vault_start_block: u64,

    /// Apibara API key for blockchain indexing
    #[arg(long, env = "APIBARA_API_KEY")]
    pub apibara_api_key: String,

    /// Vault address to index (hex format)
    #[arg(long, env = "EXTENDED_VAULT_ADRESS")]
    pub extended_vault_adress: String,
}
