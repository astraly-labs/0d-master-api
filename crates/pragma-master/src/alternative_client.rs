use anyhow::{Result as AnyResult, anyhow};
use pragma_db::types::Timeframe;
use reqwest::Client;
use serde::Deserialize;

use crate::{
    client::http_client,
    dto::{AprBasis, AprPoint, AprSeriesDTO, AprSummaryDTO, GetStatsDTO},
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
}

pub struct VaultAlternativeAPIClient {
    http_client: Client,
    api_endpoint: String,
    contract_address: String,
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
        let tvl = vault.tvl.unwrap_or("0".to_string());
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
            "all" => Timeframe::All,
            _ => Timeframe::All,
        };

        Ok(AprSeriesDTO {
            timeframe: timeframe_enum,
            points: Vec::<AprPoint>::new(),
        })
    }

    async fn fetch_all_vaults(&self) -> AnyResult<Vec<AlternativeVaultDTO>> {
        let url = format!("{}/vaults", self.api_endpoint);
        let response = self.http_client.get(url).send().await?;

        if !response.status().is_success() {
            return Err(anyhow!(
                "alternative vault API error: {}",
                response.status()
            ));
        }

        let vaults = response.json::<Vec<AlternativeVaultDTO>>().await?;
        Ok(vaults)
    }
}

fn parse_decimal(value: Option<&str>) -> f64 {
    value
        .unwrap_or("0")
        .trim()
        .parse::<f64>()
        .unwrap_or_default()
}
