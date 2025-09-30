mod quoted_assets;

use std::{sync::Arc, time::Duration};

use anyhow::Result;
use futures::future::join_all;
use pragma_common::services::{Service, ServiceRunner};
use pragma_rs::{AggregationMode, GetEntryParams, Interval, PragmaClient};
use tokio::time;

pub use quoted_assets::QuotedAssets;

/// Service responsible of quoting a list of assets in dollars.
/// The prices are medians fetched from the Pragma API.
#[derive(Clone)]
pub struct QuotingService {
    pragma_client: Arc<PragmaClient>,
    quoted_assets: Arc<QuotedAssets>,
    tracked_assets: Vec<String>,
}

impl QuotingService {
    pub fn new(
        pragma_api_env: pragma_rs::Environment,
        pragma_api_key: String,
        quoted_assets: Arc<QuotedAssets>,
        tracked_assets: Vec<String>,
    ) -> anyhow::Result<Self> {
        let pragma_client =
            PragmaClient::new(pragma_rs::Config::new(pragma_api_key, pragma_api_env))?;
        Ok(Self {
            pragma_client: Arc::new(pragma_client),
            quoted_assets,
            tracked_assets,
        })
    }

    /// Fetch prices forever from the Pragma API and store them in `QuotedAssets` field
    /// of the `AppState`.
    async fn run_forever(self) -> Result<()> {
        const PRICES_UPDATE_INTERVAL: Duration = Duration::from_secs(60);
        let mut interval = time::interval(PRICES_UPDATE_INTERVAL);

        loop {
            interval.tick().await;
            self.update_all_prices().await?;
        }
    }

    /// Update the internal state with the latest prices
    async fn update_all_prices(&self) -> Result<()> {
        const GET_ENTRY_PARAMS: Option<GetEntryParams> = Some(GetEntryParams {
            timestamp: None,
            interval: Some(Interval::OneMinute),
            aggregation: Some(AggregationMode::Median),
            routing: Some(false),
            entry_type: None,
            with_components: None,
        });

        let fetch_results = join_all(self.tracked_assets.iter().map(|coin| {
            let pragma_client = &self.pragma_client;
            async move {
                match pragma_client.get_entry(coin, "USD", GET_ENTRY_PARAMS).await {
                    Ok(response) => (coin, Ok(response)),
                    Err(e) => (coin, Err(e)),
                }
            }
        }))
        .await;

        for (coin, result) in fetch_results {
            match result {
                Ok(response) => {
                    let price = response.price_u128()?;
                    self.quoted_assets.insert(coin, (price, response.decimals));
                }
                Err(e) => {
                    tracing::debug!("Failed to fetch price for {coin}: {e}");
                }
            }
        }

        Ok(())
    }
}

#[async_trait::async_trait]
impl Service for QuotingService {
    async fn start<'a>(&mut self, mut runner: ServiceRunner<'a>) -> anyhow::Result<()> {
        let service = self.clone();

        runner.spawn_loop(|ctx| async move {
            tracing::info!(
                "🧩 Quoting service started for coins: {}",
                service.tracked_assets.join(", ")
            );

            if let Some(result) = ctx.run_until_cancelled(service.run_forever()).await {
                result?;
            }

            anyhow::Ok(())
        });

        Ok(())
    }
}
