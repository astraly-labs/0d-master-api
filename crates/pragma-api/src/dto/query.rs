use serde::Deserialize;
use utoipa::ToSchema;

/// Common query parameters for endpoints that accept timeframe
#[derive(Debug, Deserialize, ToSchema)]
pub struct TimeframeQuery {
    #[serde(default = "defaults::timeframe")]
    pub timeframe: String,
}

/// Query parameters for APR summary endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct AprSummaryQuery {
    #[serde(default = "defaults::apr_basis")]
    pub apr_basis: String,
}

/// Query parameters for APR series endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct AprSeriesQuery {
    #[serde(default = "defaults::timeframe")]
    pub timeframe: String,
}

/// Query parameters for timeseries endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct TimeseriesQuery {
    pub metric: String,
    #[serde(default = "defaults::timeframe")]
    pub timeframe: String,
    #[serde(default = "defaults::currency")]
    pub currency: String,
}

/// Query parameters for composition endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct CompositionQuery {
    #[serde(default = "default_composition_group_by")]
    pub group_by: String,
}

fn default_composition_group_by() -> String {
    "platform".to_string()
}

/// Query parameters for composition series endpoint
#[derive(Debug, Deserialize, ToSchema)]
pub struct CompositionSeriesQuery {
    #[serde(default = "defaults::timeframe")]
    pub timeframe: String,
    #[serde(default = "default_composition_group_by")]
    pub group_by: String,
}

pub mod defaults {
    pub fn timeframe() -> String {
        "all".to_string()
    }

    pub fn currency() -> String {
        "USD".to_string()
    }

    pub fn apr_basis() -> String {
        "nominal".to_string()
    }
}
