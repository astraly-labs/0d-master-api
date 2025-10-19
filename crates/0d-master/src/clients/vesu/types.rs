use crate::dto::{
    AprBasis, AprPoint, AprSeriesDTO, AprSummaryDTO, GetStatsDTO, NavLatestDTO, TimeseriesPoint,
    TimeseriesResponseDTO,
};
use chrono::{DateTime, Utc};
use zerod_db::types::Timeframe;

// ============================================================================
// Helper functions for parsing and conversion
// ============================================================================

fn parse_decimal(value: Option<&str>) -> f64 {
    value
        .unwrap_or("0")
        .trim()
        .parse::<f64>()
        .unwrap_or_default()
}

fn diff_in_pct_between_f64_string(current: Option<&str>, previous: Option<&str>) -> f64 {
    let current_value = parse_decimal(current);
    let previous_value = parse_decimal(previous);

    if previous_value.abs() > f64::EPSILON {
        ((current_value - previous_value) / previous_value) * 100.0
    } else {
        0.0
    }
}

fn to_rfc3339(timestamp: Option<f64>) -> Option<String> {
    let timestamp = timestamp? as i64;
    DateTime::<Utc>::from_timestamp(timestamp, 0).map(|dt| dt.to_rfc3339())
}

pub(super) fn timeframe_from_str(timeframe: &str) -> Timeframe {
    match timeframe {
        "7d" => Timeframe::SevenDays,
        "30d" => Timeframe::ThirtyDays,
        "1y" => Timeframe::OneYear,
        _ => Timeframe::All,
    }
}

pub(super) fn apr_basis_from_str(basis: &str) -> AprBasis {
    match basis {
        "inflation_adjusted" => AprBasis::InflationAdjusted,
        _ => AprBasis::Nominal,
    }
}

// ============================================================================
// Stats conversions
// ============================================================================

impl From<&vesu_sdk::types::VaultsControllerGetVaultsResponseItem> for GetStatsDTO {
    fn from(vault: &vesu_sdk::types::VaultsControllerGetVaultsResponseItem) -> Self {
        let tvl = vault.tvl.clone().unwrap_or_else(|| "0".to_string());
        let tvl_usd = vault.tvl_usd.clone().unwrap_or_else(|| "0".to_string());
        let apr_pct = parse_decimal(vault.apr.as_deref());

        GetStatsDTO {
            tvl,
            tvl_usd,
            past_month_apr_pct: apr_pct,
            projected_apr_pct: 6.0,
        }
    }
}

// ============================================================================
// APR Summary conversions
// ============================================================================

pub(super) struct VaultWithBasis<'a> {
    pub vault: &'a vesu_sdk::types::VaultsControllerGetVaultsResponseItem,
    pub apr_basis: AprBasis,
}

impl<'a> From<VaultWithBasis<'a>> for AprSummaryDTO {
    fn from(vwb: VaultWithBasis<'a>) -> Self {
        let apr_pct = parse_decimal(vwb.vault.apr.as_deref());

        AprSummaryDTO {
            apr_pct,
            apr_basis: vwb.apr_basis,
        }
    }
}

// ============================================================================
// APR Series conversions
// ============================================================================

// Helper to convert history entry to AprPoint (returns None if timestamp missing)
fn try_apr_point_from_entry(
    entry: &vesu_sdk::types::VaultsControllerGetVaultHistoryResponseItem,
) -> Option<AprPoint> {
    let t = to_rfc3339(entry.timestamp)?;
    let apr_pct = parse_decimal(entry.apr.as_deref());
    Some(AprPoint { t, apr_pct })
}

pub(super) struct HistoryWithTimeframe {
    pub history: Vec<vesu_sdk::types::VaultsControllerGetVaultHistoryResponseItem>,
    pub timeframe: Timeframe,
}

impl From<HistoryWithTimeframe> for AprSeriesDTO {
    fn from(hwt: HistoryWithTimeframe) -> Self {
        let points = hwt
            .history
            .iter()
            .filter_map(try_apr_point_from_entry)
            .collect();

        AprSeriesDTO {
            timeframe: hwt.timeframe,
            points,
        }
    }
}

// ============================================================================
// NAV Latest conversions
// ============================================================================

pub(super) struct NavHistory(pub Vec<vesu_sdk::types::VaultsControllerGetVaultHistoryResponseItem>);

impl From<NavHistory> for Option<NavLatestDTO> {
    fn from(nav_history: NavHistory) -> Self {
        let mut history = nav_history.0;

        if history.is_empty() {
            return None;
        }

        history.sort_by(|a, b| {
            let b_ts = b.timestamp.unwrap_or(0.0);
            let a_ts = a.timestamp.unwrap_or(0.0);
            b_ts.partial_cmp(&a_ts).unwrap_or(std::cmp::Ordering::Equal)
        });

        let current = &history[0];
        let apr_since_prev_pct = diff_in_pct_between_f64_string(
            current.apr.as_deref(),
            history.get(1).and_then(|prev| prev.apr.as_deref()),
        );
        let var_since_prev_pct = diff_in_pct_between_f64_string(
            current.tvl_usd.as_deref(),
            history.get(1).and_then(|prev| prev.tvl_usd.as_deref()),
        );
        let date = to_rfc3339(current.timestamp).unwrap_or_else(|| Utc::now().to_rfc3339());

        Some(NavLatestDTO {
            date,
            aum: current
                .tvl_usd
                .as_deref()
                .or(current.tvl.as_deref())
                .unwrap_or("0")
                .to_string(),
            var_since_prev_pct,
            apr_since_prev_pct,
            report_url: None,
        })
    }
}

// ============================================================================
// Timeseries conversions
// ============================================================================

fn try_timeseries_point_from_entry(
    entry: &vesu_sdk::types::VaultsControllerGetVaultHistoryResponseItem,
) -> Option<TimeseriesPoint> {
    let t = to_rfc3339(entry.timestamp)?;
    let v = entry.tvl.clone().unwrap_or_else(|| "0".to_string());
    Some(TimeseriesPoint { t, v })
}

pub(super) struct HistoryWithMetric {
    pub history: Vec<vesu_sdk::types::VaultsControllerGetVaultHistoryResponseItem>,
    pub metric: String,
    pub timeframe: String,
}

impl From<HistoryWithMetric> for TimeseriesResponseDTO {
    fn from(hwm: HistoryWithMetric) -> Self {
        let metric_lower = hwm.metric.to_ascii_lowercase();

        let points = if metric_lower == "tvl" {
            hwm.history
                .iter()
                .filter_map(try_timeseries_point_from_entry)
                .collect()
        } else {
            Vec::new() // pnl or other metrics not supported
        };

        TimeseriesResponseDTO {
            metric: hwm.metric,
            timeframe: hwm.timeframe,
            points,
        }
    }
}

// ============================================================================
// History parameters helper
// ============================================================================

pub(super) struct HistoryParams {
    pub max_reports: u32,
    pub apr_calculation_delta: u32,
}

pub(super) fn history_params(timeframe: &str) -> HistoryParams {
    match timeframe {
        "7d" => HistoryParams {
            max_reports: 20,
            apr_calculation_delta: 7,
        },
        "30d" => HistoryParams {
            max_reports: 60,
            apr_calculation_delta: 7,
        },
        "1y" => HistoryParams {
            max_reports: 120,
            apr_calculation_delta: 30,
        },
        _ => HistoryParams {
            max_reports: 200,
            apr_calculation_delta: 30,
        },
    }
}
