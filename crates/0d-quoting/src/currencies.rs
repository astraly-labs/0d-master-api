use std::{
    str::FromStr,
    sync::{Arc, LazyLock},
    time::Duration,
};

use anyhow::Result;
use moka::future::Cache;
use rust_decimal::Decimal;
use serde::{Deserialize, Serialize};
use utoipa::ToSchema;

use crate::pyth::fetch_pyth_price;

pub static CURRENCIES_PRICES: LazyLock<Arc<CurrenciesPrices>> =
    LazyLock::new(|| Arc::new(CurrenciesPrices::new()));

#[derive(Debug, Clone, Copy, Serialize, Deserialize, ToSchema, Hash, Eq, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Currency {
    USD,
    USDC,
}

/// Cached currency prices with 10-second TTL
#[derive(Debug)]
pub struct CurrenciesPrices(Cache<Currency, Decimal>);

impl CurrenciesPrices {
    pub fn new() -> Self {
        const CACHE_DURATION: Duration = Duration::from_secs(10);

        Self(Cache::builder().time_to_live(CACHE_DURATION).build())
    }

    /// Try to fetch the price of the given Currency. Will be always quoted in USD.
    /// The primary source is Pyth (with caching), but if it fails for any reason we will instead
    /// use the Starknet L2 price.
    pub async fn of(&self, currency: Currency) -> Result<Decimal> {
        if matches!(currency, Currency::USD) {
            return Ok(Decimal::ONE);
        }

        // Try to get from cache first
        if let Some(cached_price) = self.0.get(&currency).await {
            return Ok(cached_price);
        }

        // Not in cache, try to fetch from Pyth
        if let Ok(pyth_price) = fetch_pyth_price(currency).await {
            self.0.insert(currency, pyth_price).await;
            return Ok(pyth_price);
        }

        Err(anyhow::anyhow!("Failed to fetch price for {:?}", currency))
    }

    /// See `of`.
    /// Same but for an arbitrary ticker. Will fail if the ticker is not supported.
    /// (Not supported = not in the `Currency` enum).
    pub async fn of_ticker(&self, ticker: &str) -> Result<Decimal> {
        let currency = Currency::from_str(ticker)?;
        self.of(currency).await
    }
}

impl Default for CurrenciesPrices {
    fn default() -> Self {
        Self::new()
    }
}

impl FromStr for Currency {
    type Err = anyhow::Error;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_ascii_uppercase().as_str() {
            "USD" => Ok(Self::USD),
            "USDC" => Ok(Self::USDC),
            _ => Err(anyhow::anyhow!("Unsupported currency")),
        }
    }
}
