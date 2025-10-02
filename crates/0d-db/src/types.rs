use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum PerformanceMetric {
    AllTimePnl,
    UnrealizedPnl,
    RealizedPnl,
}

impl PerformanceMetric {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::AllTimePnl => "all_time_pnl",
            Self::UnrealizedPnl => "unrealized_pnl",
            Self::RealizedPnl => "realized_pnl",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Timeframe {
    #[serde(rename = "7d")]
    SevenDays,
    #[serde(rename = "30d")]
    ThirtyDays,
    #[serde(rename = "1y")]
    OneYear,
    #[default]
    All,
}

impl Timeframe {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::SevenDays => "7d",
            Self::ThirtyDays => "30d",
            Self::OneYear => "1y",
            Self::All => "all",
        }
    }

    pub const fn to_days(&self) -> Option<i64> {
        match self {
            Self::SevenDays => Some(7),
            Self::ThirtyDays => Some(30),
            Self::OneYear => Some(365),
            Self::All => None,
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "snake_case")]
pub enum AprBasis {
    #[default]
    Nominal,
    InflationAdjusted,
}

impl AprBasis {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Nominal => "nominal",
            Self::InflationAdjusted => "inflation_adjusted",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum GroupBy {
    #[default]
    Platform,
    Asset,
}

impl GroupBy {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Platform => "platform",
            Self::Asset => "asset",
        }
    }
}

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Metric {
    Tvl,
    Pnl,
}

impl Metric {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Tvl => "tvl",
            Self::Pnl => "pnl",
        }
    }
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, ToSchema)]
pub enum Currency {
    #[default]
    #[serde(rename = "USD")]
    Usd,
    #[serde(rename = "USDC")]
    Usdc,
}

impl Currency {
    pub const fn as_str(&self) -> &'static str {
        match self {
            Self::Usd => "USD",
            Self::Usdc => "USDC",
        }
    }
}
