use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;

use crate::errors::ApiError;

#[derive(Debug, Deserialize)]
pub struct GetStatsDTO {
    pub tvl: String,
    pub past_month_apr_pct: f64,
}

#[derive(Debug, Deserialize)]
pub struct NavLatestDTO {
    pub date: String,
    pub aum: String,
    pub var_since_prev_pct: f64,
    pub apr_since_prev_pct: f64,
    pub report_url: String,
}

#[derive(Debug, Deserialize)]
pub struct KpisDTO {
    pub cumulative_pnl_usd: String,
    pub max_drawdown_pct: f64,
    pub sharpe: f64,
    pub profit_share_bps: u32,
}

#[derive(Debug, Deserialize)]
pub struct AprSeriesPoint {
    pub t: String,    // RFC3339 timestamp
    pub apr_pct: f64, // APR in percent
}

#[derive(Debug, Deserialize)]
pub struct AprSeriesDTO {
    pub timeframe: Timeframe,
    pub points: Vec<AprSeriesPoint>,
}

#[derive(Debug, Deserialize)]
pub struct CompositionPosition {
    pub platform: String,
    pub asset: String,
    pub symbol: String,
    pub pct: f64,
    pub apy_est_pct: f64,
    pub icon: Option<String>,
}

#[derive(Debug, Deserialize)]
pub struct CompositionDTO {
    pub as_of: String,
    pub positions: Vec<CompositionPosition>,
}

#[derive(Debug, Deserialize)]
pub struct AprSummaryDTO {
    pub apr_pct: f64,
    pub apr_basis: AprSummaryBasis,
}

#[derive(Debug, Default, Deserialize, Clone, Copy)]
#[serde(rename_all = "snake_case")]
pub enum AprSummaryBasis {
    #[default]
    Nominal,
    InflationAdjusted,
}

#[derive(Debug, Deserialize, Clone, Copy)]
pub enum Timeframe {
    #[serde(rename = "7d")]
    D7,
    #[serde(rename = "30d")]
    D30,
    #[serde(rename = "1y")]
    Y1,
    #[serde(rename = "all")]
    All,
}

#[derive(Debug, Deserialize)]
#[allow(dead_code)]
pub struct VaultInfoResponse {
    /// Current epoch number
    current_epoch: String,

    /// The underlying currency ticker (e.g., "USDC", "USDT")
    underlying_currency: String,

    /// Total assets required for pending withdrawals (sum of all epochs)
    pending_withdrawals_assets: String,

    /// Assets under management (AUM) in underlying currency
    aum: String,

    /// Current buffer amount in underlying currency
    buffer: String,

    /// Current share price in USD
    pub share_price_in_usd: String,
}

pub struct VaultMasterAPIClient {
    http_client: Client,
    api_endpoint: String,
}

// TODO: use macro
impl VaultMasterAPIClient {
    pub fn new(api_endpoint: &str) -> Result<Self, ApiError> {
        let http_client = http_client().map_err(|_| ApiError::InternalServerError)?;

        Ok(Self {
            http_client,
            api_endpoint: api_endpoint.to_string(),
        })
    }

    pub async fn get_vault_stats(&self) -> anyhow::Result<GetStatsDTO> {
        let response = self
            .http_client
            .get(format!("{}/v1/master/stats", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<GetStatsDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_apr_summary(&self) -> anyhow::Result<AprSummaryDTO> {
        let response = self
            .http_client
            .get(format!("{}/v1/master/apr/summary", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<AprSummaryDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_apr_series(&self) -> anyhow::Result<AprSeriesDTO> {
        let response = self
            .http_client
            .get(format!("{}/v1/master/apr/series", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<AprSeriesDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_composition(&self) -> anyhow::Result<CompositionDTO> {
        let response = self
            .http_client
            .get(format!("{}/v1/master/composition", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<CompositionDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_nav_latest(&self) -> anyhow::Result<NavLatestDTO> {
        let response = self
            .http_client
            .get(format!("{}/v1/master/nav/latest", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<NavLatestDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_kpis(&self) -> anyhow::Result<KpisDTO> {
        let response = self
            .http_client
            .get(format!("{}/v1/master/kpis", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<KpisDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_timeseries(&self) -> anyhow::Result<AprSeriesDTO> {
        let response = self
            .http_client
            .get(format!("{}/v1/master/timeseries", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<AprSeriesDTO>().await?;
        Ok(body)
    }

    // TODO: should be in master
    pub async fn get_vault_share_price(&self) -> anyhow::Result<VaultInfoResponse> {
        let response = self
            .http_client
            .get(format!("{}/v1/vault/info", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<VaultInfoResponse>().await?;
        Ok(body)
    }
}

pub fn http_client() -> Result<Client, ApiError> {
    Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| {
            tracing::error!("Failed to build HTTP client: {}", e);
            ApiError::InternalServerError
        })
}

pub fn map_status(status: &str) -> String {
    match status {
        "active" => "live".to_string(),
        other => other.to_string(),
    }
}
