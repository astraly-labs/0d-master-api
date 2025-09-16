use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultListItem {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub chain: String,
    pub symbol: String,
    pub tvl: String,
    pub status: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultListResponse {
    pub items: Vec<VaultListItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Vault {
    pub id: String,
    pub name: String,
    pub description: Option<String>,
    pub chain: String,
    pub chain_id: Option<String>,
    pub symbol: String,
    pub base_asset: String,
    pub status: String,
    pub inception_date: Option<String>,
    pub contract_address: String,
    pub mgmt_fee_bps: Option<i32>,
    pub perf_fee_bps: i32,
    pub strategy_brief: Option<String>,
    pub docs_url: Option<String>,
    pub min_deposit: Option<String>,
    pub max_deposit: Option<String>,
    pub deposit_paused: Option<bool>,
    pub instant_liquidity: Option<bool>,
    pub instant_slippage_max_bps: Option<i32>,
    pub redeem_24h_threshold_pct_of_aum: Option<String>,
    pub redeem_48h_above_threshold: Option<bool>,
    pub icon_light_url: Option<String>,
    pub icon_dark_url: Option<String>,
    pub api_endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultStats {
    pub tvl: String,
    pub past_month_apr_pct: f64,
}

impl From<pragma_db::models::Vault> for Vault {
    fn from(vault: pragma_db::models::Vault) -> Self {
        Vault {
            id: vault.id,
            name: vault.name,
            description: vault.description,
            chain: vault.chain,
            chain_id: vault.chain_id,
            symbol: vault.symbol,
            base_asset: vault.base_asset,
            status: vault.status,
            inception_date: vault.inception_date.map(|d| d.to_string()),
            contract_address: vault.contract_address,
            mgmt_fee_bps: vault.mgmt_fee_bps,
            perf_fee_bps: vault.perf_fee_bps,
            strategy_brief: vault.strategy_brief,
            docs_url: vault.docs_url,
            min_deposit: vault.min_deposit.map(|d| d.to_string()),
            max_deposit: vault.max_deposit.map(|d| d.to_string()),
            deposit_paused: vault.deposit_paused,
            instant_liquidity: vault.instant_liquidity,
            instant_slippage_max_bps: vault.instant_slippage_max_bps,
            redeem_24h_threshold_pct_of_aum: vault.redeem_24h_threshold_pct_of_aum.map(|d| d.to_string()),
            redeem_48h_above_threshold: vault.redeem_48h_above_threshold,
            icon_light_url: vault.icon_light_url,
            icon_dark_url: vault.icon_dark_url,
            api_endpoint: vault.api_endpoint,
        }
    }
}