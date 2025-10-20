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
    pub underlying_currency: String,
    pub apr: String,
    pub status: String,
    pub average_redeem_delay: Option<String>,
    pub last_reported: Option<String>,
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
    pub inception_date: Option<String>,
    pub chain: String,
    pub chain_id: Option<String>,
    pub symbol: String,
    pub base_asset: String,
    pub status: String,
    // Live fields embedded directly
    pub tvl: String,
    pub share_price: String,
    pub mgmt_fee_bps: Option<i32>,
    pub perf_fee_bps: i32,
    pub strategy_brief: Option<String>,
    pub docs_url: Option<String>,
    pub contract: Contract,
    pub deposit_constraints: DepositConstraints,
    pub withdraw_constraints: WithdrawConstraints,
    pub icons: Icons,
    pub api_endpoint: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Contract {
    pub address: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct DepositConstraints {
    pub min_deposit: Option<String>,
    pub max_deposit: Option<String>,
    pub paused: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct WithdrawConstraints {
    pub instant_liquidity: Option<bool>,
    pub instant_slippage_max_bps: Option<i32>,
    pub redeem_24h_threshold_pct_of_aum: Option<f64>,
    pub redeem_48h_above_threshold: Option<bool>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct Icons {
    pub light: Option<String>,
    pub dark: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TimeseriesMetric {
    Tvl,
    Pnl,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum TimeseriesCurrency {
    #[serde(rename = "USD")]
    Usd,
    #[serde(rename = "USDC")]
    Usdc,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LiquiditySimulateRequest {
    pub amount: String,
}

impl From<zerod_db::models::Vault> for Vault {
    fn from(vault: zerod_db::models::Vault) -> Self {
        Self {
            id: vault.id,
            name: vault.name,
            description: vault.description,
            inception_date: vault.inception_date.map(|d| d.to_string()),
            chain: vault.chain,
            chain_id: vault.chain_id,
            symbol: vault.symbol,
            base_asset: vault.base_asset,
            status: vault.status,
            tvl: "0".to_string(),
            share_price: "0".to_string(),
            mgmt_fee_bps: vault.mgmt_fee_bps,
            perf_fee_bps: vault.perf_fee_bps,
            strategy_brief: vault.strategy_brief,
            docs_url: vault.docs_url,
            contract: Contract {
                address: vault.contract_address,
            },
            deposit_constraints: DepositConstraints {
                min_deposit: vault.min_deposit.map(|d| d.to_string()),
                max_deposit: vault.max_deposit.map(|d| d.to_string()),
                paused: vault.deposit_paused,
            },
            withdraw_constraints: WithdrawConstraints {
                instant_liquidity: vault.instant_liquidity,
                instant_slippage_max_bps: vault.instant_slippage_max_bps,
                redeem_24h_threshold_pct_of_aum: vault
                    .redeem_24h_threshold_pct_of_aum
                    .and_then(|b| b.to_string().parse::<f64>().ok()),
                redeem_48h_above_threshold: vault.redeem_48h_above_threshold,
            },
            icons: Icons {
                light: vault.icon_light_url,
                dark: vault.icon_dark_url,
            },
            api_endpoint: vault.api_endpoint,
        }
    }
}
