use std::time::Duration;

use reqwest::Client;
use serde::Deserialize;

use crate::errors::ApiError;

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

// Optional: fetch the full portfolio/stats payload for a vault.
// Useful when callers need more than TVL/APR (e.g., balances, exposure, etc.).
pub async fn fetch_vault_portfolio(
    client: &Client,
    api_endpoint: &str,
) -> Option<crate::dto::VaultPortfolioDTO> {
    let url = format!("{}/portfolio", api_endpoint.trim_end_matches('/'));
    match client.get(&url).send().await {
        Ok(resp) if resp.status().is_success() => {
            match resp.json::<crate::dto::VaultPortfolioDTO>().await {
                Ok(p) => Some(p),
                Err(e) => {
                    tracing::warn!(error = %e, url = %url, "Failed to parse portfolio JSON");
                    None
                }
            }
        }
        Ok(resp) => {
            tracing::warn!(status = %resp.status().as_u16(), url = %url, "Portfolio request non-success status");
            None
        }
        Err(e) => {
            tracing::warn!(error = %e, url = %url, "Failed to fetch portfolio");
            None
        }
    }
}

/// Convert a fraction string (e.g., "0.0447") into a percentage (4.47).
/// Returns None for invalid/NA/empty inputs.
pub fn fraction_str_to_pct_opt<S: AsRef<str>>(s: S) -> Option<f64> {
    let st = s.as_ref().trim();
    if st.is_empty() || st.eq_ignore_ascii_case("na") {
        return None;
    }
    st.parse::<f64>().map_or(None, |v| Some(v * 100.0))
}

pub fn map_status(status: &str) -> String {
    match status {
        "active" => "live".to_string(),
        other => other.to_string(),
    }
}
