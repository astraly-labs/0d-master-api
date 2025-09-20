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

#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
#[serde(rename_all = "lowercase")]
pub enum Timeframe {
    #[serde(rename = "7d")]
    SevenDays,
    #[serde(rename = "30d")]
    ThirtyDays,
    #[serde(rename = "1y")]
    OneYear,
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
