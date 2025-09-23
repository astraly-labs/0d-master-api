use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

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
    pub asset: String,
    pub symbol: String,
    pub pct: f64,
    pub apy_est_pct: f64,
    pub icon: Option<String>,
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
