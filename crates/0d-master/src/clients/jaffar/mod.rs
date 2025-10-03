mod types;

use self::types::{apr_basis_from_str, group_by_from_str, timeframe_from_str};
use crate::{
    dto::{
        AprSeriesDTO, AprSummaryDTO, CapsDTO, CompositionDTO, CompositionSeriesDTO, GetStatsDTO,
        KpisDTO, LiquidityDTO, LiquiditySimulateResponseDTO, NavLatestDTO, SlippageCurveDTO,
        TimeseriesResponseDTO, VaultInfoResponse,
    },
    error::MasterApiError,
    traits::VaultMasterClient,
};

pub struct JaffarClient {
    client: jaffar_sdk::Client,
}

impl JaffarClient {
    pub fn new(api_endpoint: &str) -> Self {
        Self {
            client: jaffar_sdk::Client::new(api_endpoint),
        }
    }
}

#[async_trait::async_trait]
impl VaultMasterClient for JaffarClient {
    async fn get_vault_stats(&self) -> Result<GetStatsDTO, MasterApiError> {
        let response = self.client.get_master_stats().await?;
        Ok(response.into_inner().into())
    }

    async fn get_vault_apr_summary(&self, apr_basis: &str) -> Result<AprSummaryDTO, MasterApiError> {
        let basis = apr_basis_from_str(apr_basis);
        let response = self.client.get_master_apr_summary(basis).await?;
        Ok(response.into_inner().into())
    }

    async fn get_vault_apr_series(&self, timeframe: &str) -> Result<AprSeriesDTO, MasterApiError> {
        let tf = timeframe_from_str(timeframe);
        let response = self.client.get_master_apr_series(tf).await?;
        Ok(response.into_inner().into())
    }

    async fn get_vault_composition(&self, group_by: &str) -> Result<CompositionDTO, MasterApiError> {
        let group_by = group_by_from_str(group_by);
        let response = self.client.get_master_composition(group_by).await?;
        Ok(response.into_inner().into())
    }

    async fn get_vault_composition_series(
        &self,
        _timeframe: &str,
        _group_by: &str,
    ) -> Result<CompositionSeriesDTO, MasterApiError> {
        // Not implemented in SDK - return error
        Err(MasterApiError::JaffarSdkError(
            "Composition series endpoint not available in Jaffar SDK".to_string(),
        ))
    }

    async fn get_vault_nav_latest(&self) -> Result<NavLatestDTO, MasterApiError> {
        let response = self.client.get_master_nav_latest().await?;
        Ok(response.into_inner().into())
    }

    async fn get_vault_caps(&self) -> Result<CapsDTO, MasterApiError> {
        // Not implemented in SDK - return error
        Err(MasterApiError::JaffarSdkError(
            "Caps endpoint not available in Jaffar SDK".to_string(),
        ))
    }

    async fn get_vault_kpis(&self, timeframe: &str) -> Result<KpisDTO, MasterApiError> {
        let tf = timeframe_from_str(timeframe);
        let response = self.client.get_master_kpis(tf).await?;
        let inner = response.into_inner();

        Ok(KpisDTO {
            cumulative_pnl_usd: inner.cumulative_pnl_usd,
            max_drawdown_pct: inner.max_drawdown_pct,
            sharpe: inner.sharpe,
            profit_share_bps: inner.profit_share_bps as u32,
        })
    }

    async fn get_vault_timeseries(
        &self,
        metric: &str,
        timeframe: &str,
        currency: &str,
    ) -> Result<TimeseriesResponseDTO, MasterApiError> {
        let tf = timeframe_from_str(timeframe);
        let metric_enum = match metric.to_lowercase().as_str() {
            "tvl" => jaffar_sdk::types::Metric::Tvl,
            "pnl" => jaffar_sdk::types::Metric::Pnl,
            _ => {
                return Err(MasterApiError::JaffarSdkError(format!(
                    "Invalid metric: {}",
                    metric
                )))
            }
        };

        let response = self
            .client
            .get_master_timeseries(Some(currency), metric_enum, tf)
            .await?;

        Ok(response.into_inner().into())
    }

    async fn get_vault_liquidity(&self) -> Result<LiquidityDTO, MasterApiError> {
        // Not implemented in SDK - return error
        Err(MasterApiError::JaffarSdkError(
            "Liquidity endpoint not available in Jaffar SDK".to_string(),
        ))
    }

    async fn get_vault_slippage_curve(&self) -> Result<SlippageCurveDTO, MasterApiError> {
        // Not implemented in SDK - return error
        Err(MasterApiError::JaffarSdkError(
            "Slippage curve endpoint not available in Jaffar SDK".to_string(),
        ))
    }

    async fn simulate_liquidity(
        &self,
        _amount: &str,
    ) -> Result<LiquiditySimulateResponseDTO, MasterApiError> {
        // Not implemented in SDK - return error
        Err(MasterApiError::JaffarSdkError(
            "Liquidity simulation endpoint not available in Jaffar SDK".to_string(),
        ))
    }

    async fn get_vault_info(&self) -> Result<VaultInfoResponse, MasterApiError> {
        // Not implemented in SDK - return error
        Err(MasterApiError::JaffarSdkError(
            "Vault info endpoint not available in Jaffar SDK".to_string(),
        ))
    }

    async fn get_vault_share_price(&self) -> Result<String, MasterApiError> {
        // Not implemented in SDK - return error
        Err(MasterApiError::JaffarSdkError(
            "Share price endpoint not available in Jaffar SDK".to_string(),
        ))
    }
}
