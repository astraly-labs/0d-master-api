use crate::{
    dto::{
        AprSeriesDTO, AprSummaryDTO, CapsDTO, CompositionDTO, CompositionSeriesDTO, GetStatsDTO,
        KpisDTO, LiquidityDTO, LiquiditySimulateResponseDTO, NavLatestDTO, SlippageCurveDTO,
        TimeseriesResponseDTO, VaultInfoResponse,
    },
    error::MasterApiError,
};

#[async_trait::async_trait]
pub trait VaultMasterClient: Send + Sync {
    async fn get_vault_stats(&self) -> Result<GetStatsDTO, MasterApiError>;

    async fn get_vault_apr_summary(&self, apr_basis: &str) -> Result<AprSummaryDTO, MasterApiError>;

    async fn get_vault_apr_series(&self, timeframe: &str) -> Result<AprSeriesDTO, MasterApiError>;

    async fn get_vault_composition(&self, group_by: &str) -> Result<CompositionDTO, MasterApiError>;

    async fn get_vault_composition_series(
        &self,
        timeframe: &str,
        group_by: &str,
    ) -> Result<CompositionSeriesDTO, MasterApiError>;

    async fn get_vault_nav_latest(&self) -> Result<NavLatestDTO, MasterApiError>;

    async fn get_vault_caps(&self) -> Result<CapsDTO, MasterApiError>;

    async fn get_vault_kpis(&self, timeframe: &str) -> Result<KpisDTO, MasterApiError>;

    async fn get_vault_timeseries(
        &self,
        metric: &str,
        timeframe: &str,
        currency: &str,
    ) -> Result<TimeseriesResponseDTO, MasterApiError>;

    async fn get_vault_liquidity(&self) -> Result<LiquidityDTO, MasterApiError>;

    async fn get_vault_slippage_curve(&self) -> Result<SlippageCurveDTO, MasterApiError>;

    async fn simulate_liquidity(
        &self,
        amount: &str,
    ) -> Result<LiquiditySimulateResponseDTO, MasterApiError>;

    async fn get_vault_info(&self) -> Result<VaultInfoResponse, MasterApiError>;

    async fn get_vault_share_price(&self) -> Result<String, MasterApiError>;
}
