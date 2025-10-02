use std::time::Duration;

use crate::{
    dto::{
        AprSeriesDTO, AprSummaryDTO, CapsDTO, CompositionDTO, CompositionSeriesDTO, GetStatsDTO,
        KpisDTO, LiquidityDTO, LiquiditySimulateRequestDTO, LiquiditySimulateResponseDTO,
        NavLatestDTO, SlippageCurveDTO, TimeseriesResponseDTO, VaultInfoResponse,
    },
    error::MasterApiError,
};
use reqwest::Client;

pub struct VaultMasterAPIClient {
    http_client: Client,
    api_endpoint: String,
}

impl VaultMasterAPIClient {
    pub fn new(api_endpoint: &str) -> Result<Self, MasterApiError> {
        let http_client = http_client().map_err(|_| MasterApiError::InternalServerError)?;

        Ok(Self {
            http_client,
            api_endpoint: api_endpoint.to_string(),
        })
    }

    pub async fn get_vault_stats(&self) -> anyhow::Result<GetStatsDTO> {
        let response = self
            .http_client
            .get(format!("{}master/stats", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<GetStatsDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_apr_summary(&self, apr_basis: &str) -> anyhow::Result<AprSummaryDTO> {
        let response = self
            .http_client
            .get(format!(
                "{}master/apr/summary?apr_basis={}",
                self.api_endpoint, apr_basis
            ))
            .send()
            .await?;
        let body = response.json::<AprSummaryDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_apr_series(&self, timeframe: &str) -> anyhow::Result<AprSeriesDTO> {
        let response = self
            .http_client
            .get(format!(
                "{}master/apr/series?timeframe={}",
                self.api_endpoint, timeframe
            ))
            .send()
            .await?;
        let body = response.json::<AprSeriesDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_composition(&self, group_by: &str) -> anyhow::Result<CompositionDTO> {
        let response = self
            .http_client
            .get(format!(
                "{}master/composition?group_by={}",
                self.api_endpoint, group_by
            ))
            .send()
            .await?;
        let body = response.json::<CompositionDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_composition_series(
        &self,
        timeframe: &str,
        group_by: &str,
    ) -> anyhow::Result<CompositionSeriesDTO> {
        let response = self
            .http_client
            .get(format!(
                "{}master/composition/series?timeframe={}&group_by={}",
                self.api_endpoint, timeframe, group_by
            ))
            .send()
            .await?;
        let body = response.json::<CompositionSeriesDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_nav_latest(&self) -> anyhow::Result<NavLatestDTO> {
        let response = self
            .http_client
            .get(format!("{}master/nav/latest", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<NavLatestDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_caps(&self) -> anyhow::Result<CapsDTO> {
        let response = self
            .http_client
            .get(format!("{}master/caps", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<CapsDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_kpis(&self, timeframe: &str) -> anyhow::Result<KpisDTO> {
        let url = format!("{}master/kpis?timeframe={}", self.api_endpoint, timeframe);
        let response = self.http_client.get(&url).send().await?;
        let body = response.json::<KpisDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_timeseries(
        &self,
        metric: &str,
        timeframe: &str,
        currency: &str,
    ) -> anyhow::Result<TimeseriesResponseDTO> {
        let url = format!(
            "{}master/timeseries?metric={}&timeframe={}&currency={}",
            self.api_endpoint, metric, timeframe, currency
        );
        let response = self.http_client.get(url).send().await?;
        let body = response.json::<TimeseriesResponseDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_liquidity(&self) -> anyhow::Result<LiquidityDTO> {
        let response = self
            .http_client
            .get(format!("{}master/liquidity", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<LiquidityDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_slippage_curve(&self) -> anyhow::Result<SlippageCurveDTO> {
        let response = self
            .http_client
            .get(format!("{}master/liquidity/curve", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<SlippageCurveDTO>().await?;
        Ok(body)
    }

    pub async fn simulate_liquidity(
        &self,
        amount: &str,
    ) -> anyhow::Result<LiquiditySimulateResponseDTO> {
        let request_body = LiquiditySimulateRequestDTO {
            amount: amount.to_string(),
        };

        let response = self
            .http_client
            .post(format!("{}master/liquidity/simulate", self.api_endpoint))
            .json(&request_body)
            .send()
            .await?;
        let body = response.json::<LiquiditySimulateResponseDTO>().await?;
        Ok(body)
    }

    pub async fn get_vault_info(&self) -> anyhow::Result<VaultInfoResponse> {
        let response = self
            .http_client
            .get(format!("{}vault/info", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<VaultInfoResponse>().await?;
        Ok(body)
    }

    pub async fn get_vault_share_price(&self) -> anyhow::Result<String> {
        let response = self
            .http_client
            .get(format!("{}vault/info", self.api_endpoint))
            .send()
            .await?;
        let body = response.json::<VaultInfoResponse>().await?;
        Ok(body.share_price_in_usd)
    }
}

pub fn http_client() -> Result<Client, MasterApiError> {
    Client::builder()
        .timeout(Duration::from_secs(2))
        .build()
        .map_err(|e| {
            tracing::error!("Failed to build HTTP client: {}", e);
            MasterApiError::InternalServerError
        })
}
