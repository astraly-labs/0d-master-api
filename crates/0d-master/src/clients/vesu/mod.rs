mod types;

use crate::{
    CapItemDTO,
    dto::{
        AprSeriesDTO, AprSummaryDTO, CapsDTO, CompositionDTO, CompositionSeriesDTO, GetStatsDTO,
        KpisDTO, LiquidityDTO, LiquiditySimulateResponseDTO, NavLatestDTO, SlippageCurveDTO,
        TimeseriesResponseDTO, VaultInfoDTO,
    },
    error::MasterApiError,
    traits::VaultMasterClient,
};
use anyhow::anyhow;
use moka::future::Cache;
use std::{sync::LazyLock, time::Duration};
use vesu_sdk::ClientInfo;

use types::{
    HistoryWithMetric, HistoryWithTimeframe, NavHistory, VaultWithBasis, apr_basis_from_str,
    history_params, timeframe_from_str,
};

pub struct VesuClient {
    client: vesu_sdk::Client,
    contract_address: String,
}

static VAULT_CACHE: LazyLock<
    Cache<String, Vec<vesu_sdk::types::VaultsControllerGetVaultsResponseItem>>,
> = LazyLock::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(30))
        .build()
});

static HISTORY_CACHE: LazyLock<
    Cache<String, Vec<vesu_sdk::types::VaultsControllerGetVaultHistoryResponseItem>>,
> = LazyLock::new(|| {
    Cache::builder()
        .time_to_live(Duration::from_secs(30))
        .build()
});

impl VesuClient {
    pub fn new(base_url: &str, contract_address: &str) -> Result<Self, MasterApiError> {
        let client = vesu_sdk::Client::new(base_url);

        Ok(Self {
            client,
            contract_address: contract_address.to_string(),
        })
    }

    async fn get_vault(
        &self,
    ) -> Result<vesu_sdk::types::VaultsControllerGetVaultsResponseItem, MasterApiError> {
        let vaults = self.fetch_all_vaults().await?;
        let target_address = self.contract_address.to_lowercase();

        vaults
            .into_iter()
            .find(|vault| {
                vault
                    .vault
                    .as_ref()
                    .map(|v| v.to_lowercase() == target_address)
                    .unwrap_or(false)
            })
            .ok_or_else(|| {
                MasterApiError::AnyhowError(anyhow!(
                    "vault {} not found in vesu API",
                    self.contract_address
                ))
            })
    }

    async fn fetch_all_vaults(
        &self,
    ) -> Result<Vec<vesu_sdk::types::VaultsControllerGetVaultsResponseItem>, MasterApiError> {
        let cache_key = self.client.baseurl().to_string();
        if let Some(cached) = VAULT_CACHE.get(&cache_key).await {
            return Ok(cached);
        }

        let response = self.client.vaults_controller_get_vaults().await?;
        let vaults = response.into_inner();
        VAULT_CACHE.insert(cache_key, vaults.clone()).await;
        Ok(vaults)
    }

    async fn fetch_history(
        &self,
        timeframe: &str,
    ) -> Result<Vec<vesu_sdk::types::VaultsControllerGetVaultHistoryResponseItem>, MasterApiError>
    {
        let cache_key = format!(
            "{}::{}::{}",
            self.client.baseurl(),
            self.contract_address,
            timeframe
        );
        if let Some(cached) = HISTORY_CACHE.get(&cache_key).await {
            return Ok(cached);
        }

        let params = history_params(timeframe);
        let response = self
            .client
            .vaults_controller_get_vault_history(
                &self.contract_address,
                Some(params.max_reports.into()),
            )
            .await?;

        let history = response.into_inner();
        HISTORY_CACHE.insert(cache_key, history.clone()).await;
        Ok(history)
    }
}

#[async_trait::async_trait]
impl VaultMasterClient for VesuClient {
    async fn get_vault_stats(&self) -> Result<GetStatsDTO, MasterApiError> {
        let vault = self.get_vault().await?;
        Ok((&vault).into())
    }

    async fn get_vault_apr_summary(
        &self,
        apr_basis: &str,
    ) -> Result<AprSummaryDTO, MasterApiError> {
        let vault = self.get_vault().await?;
        let apr_basis = apr_basis_from_str(apr_basis);
        Ok(VaultWithBasis {
            vault: &vault,
            apr_basis,
        }
        .into())
    }

    async fn get_vault_apr_series(&self, timeframe: &str) -> Result<AprSeriesDTO, MasterApiError> {
        let history = self.fetch_history(timeframe).await?;
        let timeframe = timeframe_from_str(timeframe);
        Ok(HistoryWithTimeframe { history, timeframe }.into())
    }

    async fn get_vault_composition(
        &self,
        _group_by: &str,
    ) -> Result<CompositionDTO, MasterApiError> {
        let composition = self
            .client
            .vaults_controller_get_vault_composition(&self.contract_address)
            .await?;
        Ok(composition.into_inner().into())
    }

    async fn get_vault_composition_series(
        &self,
        _timeframe: &str,
        _group_by: &str,
    ) -> Result<CompositionSeriesDTO, MasterApiError> {
        Err(MasterApiError::AnyhowError(anyhow!(
            "Composition series not supported by Vesu API"
        )))
    }

    async fn get_vault_nav_latest(&self) -> Result<NavLatestDTO, MasterApiError> {
        let history = self.fetch_history("30d").await?;
        let nav: Option<NavLatestDTO> = NavHistory(history).into();
        nav.ok_or_else(|| {
            MasterApiError::AnyhowError(anyhow!(
                "No history available for vault {}",
                self.contract_address
            ))
        })
    }

    async fn get_vault_caps(&self) -> Result<CapsDTO, MasterApiError> {
        let vault = self.get_vault().await?;
        Ok(CapsDTO {
            items: vec![CapItemDTO {
                name: "deposit".to_string(),
                current: vault
                    .tvl
                    .ok_or_else(|| MasterApiError::AnyhowError(anyhow!("TVL not found")))?
                    .parse::<f64>()
                    .map_err(|e| MasterApiError::AnyhowError(anyhow!("Invalid TVL: {}", e)))?,
                limit: vault
                    .deposit_limit
                    .ok_or_else(|| MasterApiError::AnyhowError(anyhow!("Deposit limit not found")))?
                    .parse::<f64>()
                    .map_err(|e| {
                        MasterApiError::AnyhowError(anyhow!("Invalid deposit limit: {}", e))
                    })?,
                unit: vault.underlying_symbol.ok_or_else(|| {
                    MasterApiError::AnyhowError(anyhow!("Underlying symbol not found"))
                })?,
            }],
        })
    }

    async fn get_vault_kpis(&self, _timeframe: &str) -> Result<KpisDTO, MasterApiError> {
        Err(MasterApiError::AnyhowError(anyhow!(
            "KPIs not supported by Vesu API"
        )))
    }

    async fn get_vault_timeseries(
        &self,
        metric: &str,
        timeframe: &str,
        _currency: &str,
    ) -> Result<TimeseriesResponseDTO, MasterApiError> {
        let history = self.fetch_history(timeframe).await?;
        Ok(HistoryWithMetric {
            history,
            metric: metric.to_string(),
            timeframe: timeframe.to_string(),
        }
        .into())
    }

    async fn get_vault_liquidity(&self) -> Result<LiquidityDTO, MasterApiError> {
        Err(MasterApiError::AnyhowError(anyhow!(
            "Liquidity not supported by Vesu API"
        )))
    }

    async fn get_vault_slippage_curve(&self) -> Result<SlippageCurveDTO, MasterApiError> {
        Err(MasterApiError::AnyhowError(anyhow!(
            "Slippage curve not supported by Vesu API"
        )))
    }

    async fn simulate_liquidity(
        &self,
        _amount: &str,
    ) -> Result<LiquiditySimulateResponseDTO, MasterApiError> {
        Err(MasterApiError::AnyhowError(anyhow!(
            "Liquidity simulation not supported by Vesu API"
        )))
    }

    async fn get_vault_info(&self) -> Result<VaultInfoDTO, MasterApiError> {
        let vault = self.get_vault().await?;

        Ok(VaultInfoDTO {
            current_epoch: "0".to_string(),
            underlying_currency: vault.underlying_symbol.ok_or_else(|| {
                MasterApiError::AnyhowError(anyhow!("Underlying symbol not found"))
            })?,
            underlying_currency_address: vault.underlying_asset.ok_or_else(|| {
                MasterApiError::AnyhowError(anyhow!("Underlying asset not found"))
            })?,
            pending_withdrawals_assets: "0".to_string(),
            aum: vault
                .tvl
                .ok_or_else(|| MasterApiError::AnyhowError(anyhow!("TVL not found")))?,
            buffer: "0".to_string(),
            share_price_in_usd: vault
                .share_price
                .ok_or_else(|| MasterApiError::AnyhowError(anyhow!("Share price not found")))?,
            decimals: vault
                .decimals
                .ok_or_else(|| MasterApiError::AnyhowError(anyhow!("Decimals not found")))?
                as u8,
        })
    }
}
