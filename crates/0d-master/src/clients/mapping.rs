use crate::dto::{
    AprBasis, AprPoint, AprSeriesDTO, AprSummaryDTO, CompositionDTO, CompositionPosition,
    GetStatsDTO, NavLatestDTO, TimeseriesPoint, TimeseriesResponseDTO,
};
use zerod_db::types::Timeframe;

/// Convert jaffar_sdk::types::Timeframe to zerod_db::types::Timeframe
pub(super) fn convert_timeframe_jaffar(tf: jaffar_sdk::types::Timeframe) -> Timeframe {
    match tf {
        jaffar_sdk::types::Timeframe::X7d => Timeframe::SevenDays,
        jaffar_sdk::types::Timeframe::X30d => Timeframe::ThirtyDays,
        jaffar_sdk::types::Timeframe::X1y => Timeframe::OneYear,
        jaffar_sdk::types::Timeframe::All => Timeframe::All,
    }
}

/// Convert string timeframe to jaffar_sdk::types::Timeframe
pub(super) fn timeframe_to_jaffar(timeframe: &str) -> Option<jaffar_sdk::types::Timeframe> {
    match timeframe {
        "7d" => Some(jaffar_sdk::types::Timeframe::X7d),
        "30d" => Some(jaffar_sdk::types::Timeframe::X30d),
        "1y" => Some(jaffar_sdk::types::Timeframe::X1y),
        "all" => Some(jaffar_sdk::types::Timeframe::All),
        _ => None,
    }
}

/// Convert jaffar_sdk APR basis to our AprBasis
pub(super) fn convert_apr_basis_jaffar(basis: jaffar_sdk::types::AprSummaryBasis) -> AprBasis {
    match basis {
        jaffar_sdk::types::AprSummaryBasis::Nominal => AprBasis::Nominal,
        jaffar_sdk::types::AprSummaryBasis::InflationAdjusted => AprBasis::InflationAdjusted,
    }
}

/// Convert string APR basis to jaffar_sdk::types::AprSummaryBasis
pub(super) fn apr_basis_to_jaffar(basis: &str) -> Option<jaffar_sdk::types::AprSummaryBasis> {
    match basis {
        "nominal" => Some(jaffar_sdk::types::AprSummaryBasis::Nominal),
        "inflation_adjusted" => Some(jaffar_sdk::types::AprSummaryBasis::InflationAdjusted),
        _ => None,
    }
}

/// Convert jaffar_sdk GetStatsResponse to GetStatsDTO
pub(super) fn convert_stats_jaffar(stats: jaffar_sdk::types::GetStatsResponse) -> GetStatsDTO {
    GetStatsDTO {
        tvl: stats.tvl,
        past_month_apr_pct: stats.past_month_apr_pct,
    }
}

/// Convert jaffar_sdk AprSummaryResponse to AprSummaryDTO
pub(super) fn convert_apr_summary_jaffar(summary: jaffar_sdk::types::AprSummaryResponse) -> AprSummaryDTO {
    AprSummaryDTO {
        apr_pct: summary.apr_pct,
        apr_basis: convert_apr_basis_jaffar(summary.apr_basis),
    }
}

/// Convert jaffar_sdk AprSeriesResponse to AprSeriesDTO
pub(super) fn convert_apr_series_jaffar(series: jaffar_sdk::types::AprSeriesResponse) -> AprSeriesDTO {
    AprSeriesDTO {
        timeframe: convert_timeframe_jaffar(series.timeframe),
        points: series
            .points
            .into_iter()
            .map(|p| AprPoint {
                t: p.t,
                apr_pct: p.apr_pct,
            })
            .collect(),
    }
}

/// Convert jaffar_sdk NavLatestResponse to NavLatestDTO
pub(super) fn convert_nav_latest_jaffar(nav: jaffar_sdk::types::NavLatestResponse) -> NavLatestDTO {
    NavLatestDTO {
        date: nav.date,
        aum: nav.aum,
        var_since_prev_pct: nav.var_since_prev_pct,
        apr_since_prev_pct: nav.apr_since_prev_pct,
        report_url: nav.report_url,
    }
}

/// Convert jaffar_sdk TimeseriesResponse to TimeseriesResponseDTO
pub(super) fn convert_timeseries_jaffar(
    ts: jaffar_sdk::types::TimeseriesResponse,
) -> TimeseriesResponseDTO {
    TimeseriesResponseDTO {
        metric: ts.metric,
        timeframe: ts.timeframe.to_string(),
        points: ts
            .points
            .into_iter()
            .map(|p| TimeseriesPoint { t: p.t, v: p.v })
            .collect(),
    }
}

/// Convert string group_by to jaffar_sdk::types::CompositionGroupBy
pub(super) fn group_by_to_jaffar(group_by: &str) -> Option<jaffar_sdk::types::CompositionGroupBy> {
    match group_by.to_lowercase().as_str() {
        "platform" => Some(jaffar_sdk::types::CompositionGroupBy::Platform),
        "asset" => Some(jaffar_sdk::types::CompositionGroupBy::Asset),
        _ => None,
    }
}

/// Convert jaffar_sdk CompositionResponse to CompositionDTO
pub(super) fn convert_composition_jaffar(
    comp: jaffar_sdk::types::CompositionResponse,
) -> CompositionDTO {
    CompositionDTO {
        as_of: comp.as_of,
        positions: comp
            .positions
            .into_iter()
            .map(|p| CompositionPosition {
                platform: p.platform.clone(),
                asset: p.symbol.clone(), // Use symbol as asset (SDK doesn't have separate asset field)
                symbol: p.symbol,
                pct: p.pct,
                apy_est_pct: p.apy_est_pct,
                icon: Some(p.icon),
            })
            .collect(),
    }
}
