use anyhow::{Result as AnyResult, anyhow};
use chrono::{DateTime, Utc};
use moka::future::Cache;
use pragma_db::types::Timeframe;
use reqwest::Client;
use serde::{Deserialize, Deserializer};
use serde_json::Value;
use std::{sync::LazyLock, time::Duration};

use crate::{
    client::http_client,
    dto::{
        AprBasis, AprPoint, AprSeriesDTO, AprSummaryDTO, GetStatsDTO, NavLatestDTO,
        TimeseriesPoint, TimeseriesResponseDTO,
    },
    error::MasterApiError,
};

#[derive(Debug, Clone, Deserialize)]
pub struct AlternativeVaultDTO {
    pub vault: String,
    #[serde(default, rename = "tvl")]
    pub tvl: Option<String>,
    #[serde(default, rename = "tvlUsd")]
    pub tvl_usd: Option<String>,
    #[serde(default, rename = "sharePrice")]
    pub share_price: Option<String>,
    #[serde(default)]
    pub apr: Option<String>,
    #[serde(
        default,
        rename = "averageRedeemDelay",
        deserialize_with = "deserialize_string_option"
    )]
    pub average_redeem_delay: Option<String>,
    #[serde(
        default,
        rename = "lastReported",
        deserialize_with = "deserialize_string_option"
    )]
    pub last_reported: Option<String>,
}

pub struct VaultAlternativeAPIClient {
    http_client: Client,
    api_endpoint: String,
    contract_address: String,
}

static VAULT_CACHE: LazyLock<Cache<String, Vec<AlternativeVaultDTO>>> = LazyLock::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(30))
        .build()
});

static HISTORY_CACHE: LazyLock<Cache<String, Vec<AlternativeHistoryEntry>>> = LazyLock::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(30))
        .build()
});

#[derive(Debug, Clone, Deserialize)]
struct AlternativeHistoryEntry {
    pub timestamp: i64,
    #[serde(default, rename = "tvl")]
    pub tvl: Option<String>,
    #[serde(default, rename = "tvlUsd")]
    pub tvl_usd: Option<String>,
    #[serde(default)]
    pub apr: Option<String>,
}

impl VaultAlternativeAPIClient {
    pub fn new(api_endpoint: &str, contract_address: &str) -> Result<Self, MasterApiError> {
        let http_client = http_client().map_err(|_| MasterApiError::InternalServerError)?;

        Ok(Self {
            http_client,
            api_endpoint: api_endpoint.trim_end_matches('/').to_string(),
            contract_address: contract_address.to_string(),
        })
    }

    pub async fn get_vault(&self) -> AnyResult<AlternativeVaultDTO> {
        let vaults = self.fetch_all_vaults().await?;
        let target_address = self.contract_address.to_lowercase();

        vaults
            .into_iter()
            .find(|vault| vault.vault.to_lowercase() == target_address)
            .ok_or_else(|| {
                anyhow!(
                    "vault {} not found in alternative API",
                    self.contract_address
                )
            })
    }

    pub async fn get_vault_stats(&self) -> AnyResult<GetStatsDTO> {
        let vault = self.get_vault().await?;
        let tvl = vault.tvl.clone().unwrap_or_else(|| "0".to_string());
        let apr_pct = parse_decimal(vault.apr.as_deref());

        Ok(GetStatsDTO {
            tvl,
            past_month_apr_pct: apr_pct,
        })
    }

    pub async fn get_vault_share_price(&self) -> AnyResult<String> {
        let vault = self.get_vault().await?;
        Ok(vault.share_price.unwrap_or_else(|| "0".to_string()))
    }

    pub async fn get_vault_apr_summary(&self, apr_basis: &str) -> AnyResult<AprSummaryDTO> {
        let vault = self.get_vault().await?;
        let apr_pct = parse_decimal(vault.apr.as_deref());
        let apr_basis = match apr_basis {
            "inflation_adjusted" => AprBasis::InflationAdjusted,
            _ => AprBasis::Nominal,
        };

        Ok(AprSummaryDTO { apr_pct, apr_basis })
    }

    pub async fn get_vault_apr_series(&self, timeframe: &str) -> AnyResult<AprSeriesDTO> {
        let timeframe_enum = match timeframe {
            "7d" => Timeframe::SevenDays,
            "30d" => Timeframe::ThirtyDays,
            "1y" => Timeframe::OneYear,
            _ => Timeframe::All,
        };

        let history = self.fetch_history(timeframe).await?;
        let points = history
            .into_iter()
            .filter_map(|entry| {
                let ts = to_rfc3339(entry.timestamp)?;
                let apr_pct = parse_decimal(entry.apr.as_deref());
                Some(AprPoint { t: ts, apr_pct })
            })
            .collect();

        Ok(AprSeriesDTO {
            timeframe: timeframe_enum,
            points,
        })
    }

    pub async fn get_vault_timeseries(
        &self,
        metric: &str,
        timeframe: &str,
        currency: &str,
    ) -> AnyResult<TimeseriesResponseDTO> {
        let history = self.fetch_history(timeframe).await?;
        let metric_lower = metric.to_ascii_lowercase();
        // TODO: Implement currency support
        let _ = currency;

        let mut points = Vec::with_capacity(history.len());
        for entry in history {
            let Some(ts) = to_rfc3339(entry.timestamp) else {
                continue;
            };

            let value = match metric_lower.as_str() {
                "tvl" => entry.tvl.unwrap_or_else(|| "0".to_string()),
                "pnl" => continue,
                _ => "0".to_string(),
            };

            points.push(TimeseriesPoint { t: ts, v: value });
        }

        Ok(TimeseriesResponseDTO {
            metric: metric.to_string(),
            timeframe: timeframe.to_string(),
            points,
        })
    }

    pub async fn get_vault_nav_latest(&self) -> AnyResult<NavLatestDTO> {
        let mut history = self.fetch_history("30d").await?;
        if history.is_empty() {
            return Err(anyhow!(
                "no history available for vault {}",
                self.contract_address
            ));
        }

        history.sort_by(|a, b| b.timestamp.cmp(&a.timestamp));

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

        Ok(NavLatestDTO {
            date,
            aum: current
                .tvl_usd
                .as_deref()
                .or(current.tvl.as_deref())
                .unwrap_or("0")
                .to_string(),
            var_since_prev_pct,
            apr_since_prev_pct,
            report_url: String::new(),
        })
    }

    async fn fetch_all_vaults(&self) -> AnyResult<Vec<AlternativeVaultDTO>> {
        let endpoint = self.api_endpoint.clone();
        if let Some(cached) = VAULT_CACHE.get(&endpoint).await {
            return Ok(cached);
        }

        let client = self.http_client.clone();
        let fresh = fetch_all_vaults_uncached(client, endpoint.clone()).await?;
        VAULT_CACHE.insert(endpoint, fresh.clone()).await;
        Ok(fresh)
    }

    async fn fetch_history(&self, timeframe: &str) -> AnyResult<Vec<AlternativeHistoryEntry>> {
        let cache_key = format!(
            "{}::{}::{}",
            self.api_endpoint, self.contract_address, timeframe
        );
        if let Some(cached) = HISTORY_CACHE.get(&cache_key).await {
            return Ok(cached);
        }

        let client = self.http_client.clone();
        let endpoint = self.api_endpoint.clone();
        let contract = self.contract_address.clone();
        let timeframe_owned = timeframe.to_string();

        let fresh = fetch_history_uncached(client, endpoint, contract, timeframe_owned).await?;
        HISTORY_CACHE.insert(cache_key, fresh.clone()).await;
        Ok(fresh)
    }
}

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

async fn fetch_all_vaults_uncached(
    http_client: Client,
    api_endpoint: String,
) -> AnyResult<Vec<AlternativeVaultDTO>> {
    let url = format!("{api_endpoint}/vaults");
    let response = http_client.get(url).send().await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "alternative vault API error: {}",
            response.status()
        ));
    }

    let vaults = response.json::<Vec<AlternativeVaultDTO>>().await?;
    Ok(vaults)
}

async fn fetch_history_uncached(
    http_client: Client,
    api_endpoint: String,
    contract_address: String,
    timeframe: String,
) -> AnyResult<Vec<AlternativeHistoryEntry>> {
    let params = history_params(&timeframe);
    let url = format!("{api_endpoint}/vaults/{contract_address}/history");

    let response = http_client
        .get(url)
        .query(&[
            ("maxReports", params.max_reports.to_string()),
            (
                "aprCalculationDelta",
                params.apr_calculation_delta.to_string(),
            ),
        ])
        .send()
        .await?;

    if !response.status().is_success() {
        return Err(anyhow!(
            "alternative vault history error: {}",
            response.status()
        ));
    }

    let history = response.json::<Vec<AlternativeHistoryEntry>>().await?;
    Ok(history)
}

fn to_rfc3339(timestamp: i64) -> Option<String> {
    DateTime::<Utc>::from_timestamp(timestamp, 0).map(|dt| dt.to_rfc3339())
}

struct HistoryParams {
    max_reports: u32,
    apr_calculation_delta: u32,
}

fn history_params(timeframe: &str) -> HistoryParams {
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

fn deserialize_string_option<'de, D>(deserializer: D) -> Result<Option<String>, D::Error>
where
    D: Deserializer<'de>,
{
    let option = Option::<Value>::deserialize(deserializer)?;
    Ok(option.and_then(|value| match value {
        serde_json::Value::String(s) => Some(s),
        serde_json::Value::Number(num) => Some(num.to_string()),
        serde_json::Value::Bool(b) => Some(b.to_string()),
        serde_json::Value::Null => None,
        other => Some(other.to_string()),
    }))
}
