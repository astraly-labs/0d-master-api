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
pub struct VaultStats {
    pub tvl: String,
    pub past_month_apr_pct: f64,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum TimeseriesMetric {
    Tvl,
    Pnl,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum TimeseriesTimeframe {
    #[serde(rename = "7d")]
    D7,
    #[serde(rename = "30d")]
    D30,
    #[serde(rename = "1y")]
    Y1,
    #[serde(rename = "all")]
    All,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub enum TimeseriesCurrency {
    #[serde(rename = "USD")]
    Usd,
    #[serde(rename = "USDC")]
    Usdc,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeseriesPoint {
    pub t: String, // RFC3339 timestamp
    pub v: String, // Value as string for precision
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultTimeseriesResponse {
    pub metric: String,
    pub timeframe: String,
    pub points: Vec<TimeseriesPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultKpisResponse {
    pub cumulative_pnl_usd: String,
    pub max_drawdown_pct: f64,
    pub sharpe: f64,
    pub profit_share_bps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultLiquidityResponse {
    pub as_of: Option<String>,
    pub is_liquid: bool,
    pub withdraw_capacity_usd_24h: String,
    pub deposit_capacity_usd_24h: String,
    pub policy_markdown: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct SlippagePoint {
    pub amount_usd: String,
    pub slippage_bps: u32,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultSlippageCurveResponse {
    pub is_liquid: bool,
    pub points: Vec<SlippagePoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LiquiditySimulateRequest {
    pub amount: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct InstantLiquidity {
    pub supported: bool,
    pub est_slippage_bps: u32,
    pub cap_remaining: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct ScheduledWindow {
    pub window: String,
    pub max_without_delay: Option<String>,
    pub expected_nav_date: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct LiquiditySimulateResponse {
    pub amount: String,
    pub instant: Option<InstantLiquidity>,
    pub scheduled: Vec<ScheduledWindow>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AprBasis {
    Nominal,
    InflationAdjusted,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultAprSummaryResponse {
    pub apr_pct: f64,
    pub apr_basis: AprBasis,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AprPoint {
    pub t: String,    // RFC3339 timestamp
    pub apr_pct: f64, // APR in percent
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultAprSeriesResponse {
    pub timeframe: String,
    pub points: Vec<AprPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum GroupBy {
    Platform,
    Asset,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompositionPosition {
    pub platform: String,
    pub asset: String,
    pub symbol: String,
    pub pct: f64,
    pub apy_est_pct: f64,
    pub icon: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultCompositionResponse {
    pub as_of: String,
    pub positions: Vec<CompositionPosition>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompositionSeriesPoint {
    pub t: String,             // RFC3339 timestamp
    pub weights_pct: Vec<f64>, // Weight percentages matching labels order
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultCompositionSeriesResponse {
    pub timeframe: String,
    pub group_by: String,
    pub labels: Vec<String>,
    pub points: Vec<CompositionSeriesPoint>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CapItem {
    pub name: String,
    pub current: f64,
    pub limit: f64,
    pub unit: String,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultCapsResponse {
    pub items: Vec<CapItem>,
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultNavLatestResponse {
    pub date: String,
    pub aum: String,
    pub var_since_prev_pct: f64,
    pub apr_since_prev_pct: f64,
    pub report_url: String,
}

impl From<pragma_db::models::Vault> for Vault {
    fn from(vault: pragma_db::models::Vault) -> Self {
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
