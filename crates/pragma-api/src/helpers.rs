use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;

use crate::errors::ApiError;

#[derive(Debug, Deserialize, Clone)]
pub struct StatsResp {
    pub tvl: String,
    #[serde(default)]
    pub past_month_apr_pct: Option<f64>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct NavLatestResp {
    #[serde(default)]
    pub share_price: Option<String>,
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

pub async fn fetch_vault_stats(client: &Client, api_endpoint: &str) -> Option<StatsResp> {
    let url = format!("{}/stats", api_endpoint.trim_end_matches('/'));
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.json::<StatsResp>().await {
            Ok(s) => Some(s),
            Err(e) => {
                tracing::warn!(error = %e, url = %url, "Failed to parse stats JSON");
                None
            }
        },
        Ok(resp) => {
            tracing::warn!(status = %resp.status().as_u16(), url = %url, "Stats request non-success status");
            None
        }
        Err(e) => {
            tracing::warn!(error = %e, url = %url, "Failed to fetch stats");
            None
        }
    }
}

pub async fn fetch_vault_share_price(client: &Client, api_endpoint: &str) -> Option<String> {
    let url = format!("{}/nav/latest", api_endpoint.trim_end_matches('/'));
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => match resp.json::<NavLatestResp>().await {
            Ok(n) => n.share_price,
            Err(e) => {
                tracing::warn!(error = %e, url = %url, "Failed to parse nav/latest JSON");
                None
            }
        },
        Ok(resp) => {
            tracing::warn!(status = %resp.status().as_u16(), url = %url, "nav/latest request non-success status");
            None
        }
        Err(e) => {
            tracing::warn!(error = %e, url = %url, "Failed to fetch nav/latest");
            None
        }
    }
}

pub fn map_status(status: &str) -> String {
    match status {
        "active" => "live".to_string(),
        other => other.to_string(),
    }
}
