use crate::{
    VaultInfoResponse,
    dto::{
        AprBasis, AprPoint, AprSeriesDTO, AprSummaryDTO, CompositionDTO, CompositionPosition,
        GetStatsDTO, NavLatestDTO, TimeseriesPoint, TimeseriesResponseDTO,
    },
};

// ============================================================================
// Timeframe conversions - uses helper functions (can't impl From for external types)
// ============================================================================

pub(super) fn timeframe_from_str(timeframe: &str) -> Option<jaffar_sdk::types::Timeframe> {
    match timeframe {
        "7d" => Some(jaffar_sdk::types::Timeframe::X7d),
        "30d" => Some(jaffar_sdk::types::Timeframe::X30d),
        "1y" => Some(jaffar_sdk::types::Timeframe::X1y),
        "all" => Some(jaffar_sdk::types::Timeframe::All),
        _ => None,
    }
}

// ============================================================================
// APR Basis conversions
// ============================================================================

impl From<jaffar_sdk::types::AprSummaryBasis> for AprBasis {
    fn from(basis: jaffar_sdk::types::AprSummaryBasis) -> Self {
        match basis {
            jaffar_sdk::types::AprSummaryBasis::Nominal => AprBasis::Nominal,
            jaffar_sdk::types::AprSummaryBasis::InflationAdjusted => AprBasis::InflationAdjusted,
        }
    }
}

pub(super) fn apr_basis_from_str(basis: &str) -> Option<jaffar_sdk::types::AprSummaryBasis> {
    match basis {
        "nominal" => Some(jaffar_sdk::types::AprSummaryBasis::Nominal),
        "inflation_adjusted" => Some(jaffar_sdk::types::AprSummaryBasis::InflationAdjusted),
        _ => None,
    }
}

// ============================================================================
// GroupBy conversions
// ============================================================================

pub(super) fn group_by_from_str(group_by: &str) -> Option<jaffar_sdk::types::CompositionGroupBy> {
    match group_by.to_lowercase().as_str() {
        "platform" => Some(jaffar_sdk::types::CompositionGroupBy::Platform),
        "asset" => Some(jaffar_sdk::types::CompositionGroupBy::Asset),
        _ => None,
    }
}

// ============================================================================
// Stats conversions
// ============================================================================

impl From<jaffar_sdk::types::GetStatsResponse> for GetStatsDTO {
    fn from(stats: jaffar_sdk::types::GetStatsResponse) -> Self {
        GetStatsDTO {
            tvl: stats.tvl,
            past_month_apr_pct: stats.past_month_apr_pct,
        }
    }
}

// ============================================================================
// APR Summary conversions
// ============================================================================

impl From<jaffar_sdk::types::AprSummaryResponse> for AprSummaryDTO {
    fn from(summary: jaffar_sdk::types::AprSummaryResponse) -> Self {
        AprSummaryDTO {
            apr_pct: summary.apr_pct,
            apr_basis: summary.apr_basis.into(),
        }
    }
}

// ============================================================================
// APR Series conversions
// ============================================================================

impl From<jaffar_sdk::types::AprSeriesPoint> for AprPoint {
    fn from(p: jaffar_sdk::types::AprSeriesPoint) -> Self {
        AprPoint {
            t: p.t,
            apr_pct: p.apr_pct,
        }
    }
}

impl From<jaffar_sdk::types::AprSeriesResponse> for AprSeriesDTO {
    fn from(series: jaffar_sdk::types::AprSeriesResponse) -> Self {
        use zerod_db::types::Timeframe;

        let timeframe = match series.timeframe {
            jaffar_sdk::types::Timeframe::X7d => Timeframe::SevenDays,
            jaffar_sdk::types::Timeframe::X30d => Timeframe::ThirtyDays,
            jaffar_sdk::types::Timeframe::X1y => Timeframe::OneYear,
            jaffar_sdk::types::Timeframe::All => Timeframe::All,
        };

        AprSeriesDTO {
            timeframe,
            points: series.points.into_iter().map(Into::into).collect(),
        }
    }
}

// ============================================================================
// NAV Latest conversions
// ============================================================================

impl From<jaffar_sdk::types::NavLatestResponse> for NavLatestDTO {
    fn from(nav: jaffar_sdk::types::NavLatestResponse) -> Self {
        NavLatestDTO {
            date: nav.date,
            aum: nav.aum,
            var_since_prev_pct: nav.var_since_prev_pct,
            apr_since_prev_pct: nav.apr_since_prev_pct,
            report_url: nav.report_url,
        }
    }
}

// ============================================================================
// Timeseries conversions
// ============================================================================

impl From<jaffar_sdk::types::TimePoint> for TimeseriesPoint {
    fn from(p: jaffar_sdk::types::TimePoint) -> Self {
        TimeseriesPoint { t: p.t, v: p.v }
    }
}

impl From<jaffar_sdk::types::TimeseriesResponse> for TimeseriesResponseDTO {
    fn from(ts: jaffar_sdk::types::TimeseriesResponse) -> Self {
        TimeseriesResponseDTO {
            metric: ts.metric,
            timeframe: ts.timeframe.to_string(),
            points: ts.points.into_iter().map(Into::into).collect(),
        }
    }
}

// ============================================================================
// Composition conversions
// ============================================================================

impl From<jaffar_sdk::types::CompositionPosition> for CompositionPosition {
    fn from(p: jaffar_sdk::types::CompositionPosition) -> Self {
        CompositionPosition {
            platform: p.platform.clone(),
            asset: p.symbol.clone(), // Use symbol as asset (SDK doesn't have separate asset field)
            symbol: p.symbol,
            pct: p.pct,
            apy_est_pct: p.apy_est_pct,
            icon: Some(p.icon),
        }
    }
}

impl From<jaffar_sdk::types::CompositionResponse> for CompositionDTO {
    fn from(comp: jaffar_sdk::types::CompositionResponse) -> Self {
        CompositionDTO {
            as_of: comp.as_of,
            positions: comp.positions.into_iter().map(Into::into).collect(),
        }
    }
}

// ============================================================================
// Info conversions
// ============================================================================

impl From<jaffar_sdk::types::VaultInfoResponse> for VaultInfoResponse {
    fn from(info: jaffar_sdk::types::VaultInfoResponse) -> Self {
        VaultInfoResponse {
            current_epoch: info.current_epoch,
            underlying_currency: info.underlying_currency,
            pending_withdrawals_assets: info.pending_withdrawals_assets,
            aum: info.aum,
            buffer: info.buffer,
            share_price_in_usd: info.share_price_in_usd,
        }
    }
}
