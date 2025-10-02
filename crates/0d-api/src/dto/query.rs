use serde::Deserialize;
use utoipa::ToSchema;
use zerod_db::types::{AprBasis, Currency, GroupBy, Metric, Timeframe};

/// Common query parameters for endpoints that accept timeframe
#[derive(Debug, Deserialize, ToSchema)]
pub struct TimeframeQuery {
    #[serde(default)]
    pub timeframe: Timeframe,
}

/// Query parameters for APR summary endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct AprSummaryQuery {
    #[serde(default)]
    pub apr_basis: AprBasis,
}

/// Query parameters for APR series endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct AprSeriesQuery {
    #[serde(default)]
    pub timeframe: Timeframe,
}

/// Query parameters for timeseries endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct TimeseriesQuery {
    pub metric: Metric,
    #[serde(default)]
    pub timeframe: Timeframe,
    #[serde(default)]
    pub currency: Currency,
}

/// Query parameters for composition endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct CompositionQuery {
    #[serde(default)]
    pub group_by: GroupBy,
}

/// Query parameters for composition series endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct CompositionSeriesQuery {
    #[serde(default)]
    pub timeframe: Timeframe,
    #[serde(default)]
    pub group_by: GroupBy,
}
