use serde::{Deserialize, Serialize};
use utoipa::ToSchema;
use zerod_db::types::Timeframe;

/// Common timeseries data point used across vault and user endpoints
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct TimeseriesPoint {
    pub t: String, // RFC3339 timestamp
    pub v: String, // Value as string for precision
}

/// APR data point for timeseries
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct AprPoint {
    pub t: String,    // RFC3339 timestamp
    pub apr_pct: f64, // APR in percent
}

/// Composition position data
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct CompositionPosition {
    pub platform: String,
    pub debt_asset: String,
    pub collateral_asset: String,
    pub pct: f64,
    pub apy_est_pct: f64,
}

/// APR basis enum
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AprBasis {
    Nominal,
    InflationAdjusted,
}

/// Group by enum for composition endpoints
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum GroupBy {
    Platform,
    Asset,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct GetStatsDTO {
    pub tvl: String,
    pub tvl_usd: String,
    pub past_month_apr_pct: f64,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct NavLatestDTO {
    pub date: String,
    pub aum: String,
    pub var_since_prev_pct: f64,
    pub apr_since_prev_pct: f64,
    pub report_url: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct KpisDTO {
    pub cumulative_pnl_usd: String,
    pub max_drawdown_pct: f64,
    pub sharpe: f64,
    pub profit_share_bps: u32,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AprSeriesDTO {
    pub timeframe: Timeframe,
    pub points: Vec<AprPoint>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct TimeseriesResponseDTO {
    pub metric: String,
    pub timeframe: String,
    pub points: Vec<TimeseriesPoint>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct LiquidityDTO {
    pub as_of: Option<String>,
    pub is_liquid: bool,
    pub withdraw_capacity_usd_24h: String,
    pub deposit_capacity_usd_24h: String,
    pub policy_markdown: Option<String>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SlippagePointDTO {
    pub amount_usd: String,
    pub slippage_bps: u32,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct SlippageCurveDTO {
    pub is_liquid: bool,
    pub points: Vec<SlippagePointDTO>,
}

#[derive(Debug, Serialize)]
pub struct LiquiditySimulateRequestDTO {
    pub amount: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct InstantLiquidityDTO {
    pub supported: bool,
    pub est_slippage_bps: u32,
    pub cap_remaining: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct ScheduledWindowDTO {
    pub window: String,
    pub max_without_delay: Option<String>,
    pub expected_nav_date: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct LiquiditySimulateResponseDTO {
    pub amount: String,
    pub instant: Option<InstantLiquidityDTO>,
    pub scheduled: Vec<ScheduledWindowDTO>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CompositionDTO {
    pub as_of: String,
    pub positions: Vec<CompositionPosition>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CompositionSeriesPointDTO {
    pub t: String,             // RFC3339 timestamp
    pub weights_pct: Vec<f64>, // Weight percentages matching labels order
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CompositionSeriesDTO {
    pub timeframe: String,
    pub group_by: String,
    pub labels: Vec<String>,
    pub points: Vec<CompositionSeriesPointDTO>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CapItemDTO {
    pub name: String,
    pub current: f64,
    pub limit: f64,
    pub unit: String,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct CapsDTO {
    pub items: Vec<CapItemDTO>,
}

#[derive(Debug, Deserialize, Serialize, ToSchema)]
pub struct AprSummaryDTO {
    pub apr_pct: f64,
    pub apr_basis: AprBasis,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct VaultInfoResponse {
    /// Current epoch number
    pub current_epoch: String,

    /// The underlying currency ticker (e.g., "USDC", "USDT")
    pub underlying_currency: String,

    /// Total assets required for pending withdrawals (sum of all epochs)
    pub pending_withdrawals_assets: String,

    /// Assets under management (AUM) in underlying currency
    pub aum: String,

    /// Current buffer amount in underlying currency
    pub buffer: String,

    /// Current share price in USD
    pub share_price_in_usd: String,
}
