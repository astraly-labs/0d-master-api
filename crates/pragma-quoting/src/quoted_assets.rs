use anyhow::Context;
use dashmap::DashMap;
use pragma_common::Pair;
use rust_decimal::Decimal;
use rust_decimal::prelude::*;

type Ticker = String;

#[derive(Clone, Default, Debug)]
pub struct QuotedAssets(DashMap<Ticker, Decimal>);

impl QuotedAssets {
    pub fn get(&self, coin: &str) -> Option<Decimal> {
        self.0.get(&coin.to_uppercase()).map(|v| *v.value())
    }

    pub fn insert(&self, coin: &str, price: (u128, u32)) -> Option<Decimal> {
        let decimal_price = Decimal::new(price.0 as i64, price.1);
        if decimal_price.is_sign_negative() {
            return None;
        }
        self.0.insert(coin.to_uppercase(), decimal_price)
    }

    pub fn quote(&self, price: f64, pair: &Pair) -> anyhow::Result<u128> {
        if price < 0.0 {
            return Err(anyhow::anyhow!("Price cannot be negative"));
        }

        // Convert to Decimal
        let price_dec = Decimal::from_f64(price)
            .ok_or_else(|| anyhow::anyhow!("Failed to convert price to Decimal"))?;

        // Convert to USD
        let usd_price_dec = self.quote_in_usd(&pair.quote, price_dec)?;

        // Scale to 18 decimals
        let scaled = usd_price_dec * Decimal::from(10_u128.pow(18));

        // Convert to u128
        scaled
            .to_u128()
            .ok_or_else(|| anyhow::anyhow!("Failed to convert price to u128"))
    }

    /// Converts a decimal price quoted in a specific asset to USD
    pub fn quote_in_usd(&self, quote_asset: &str, price: Decimal) -> anyhow::Result<Decimal> {
        let quote_asset = quote_asset.to_uppercase();

        if quote_asset == "USD" {
            return Ok(price);
        }

        let asset_usd_price = self
            .get(&quote_asset)
            .with_context(|| format!("No USD price found for quote asset {quote_asset}"))?;

        // Convert the price from quote_asset to USD by multiplying by the asset's USD price
        let usd_price = price * asset_usd_price;

        Ok(usd_price)
    }
}

// Utils implementation to easily spin up QuotedAssets (in tests mainly).
impl From<Vec<(String, Decimal)>> for QuotedAssets {
    fn from(pairs: Vec<(String, Decimal)>) -> Self {
        let quoted_assets = Self::default();
        for (ticker, price) in pairs {
            if !price.is_sign_negative() {
                quoted_assets.0.insert(ticker.to_uppercase(), price);
            }
        }
        quoted_assets
    }
}

impl From<Vec<(String, (u128, u8))>> for QuotedAssets {
    fn from(pairs: Vec<(String, (u128, u8))>) -> Self {
        let quoted_assets = Self::default();
        for (ticker, (price, decimals)) in pairs {
            let decimal_price = Decimal::new(price as i64, decimals as u32);
            if !decimal_price.is_sign_negative() {
                quoted_assets.0.insert(ticker.to_uppercase(), decimal_price);
            }
        }
        quoted_assets
    }
}

impl From<Vec<(String, (u128, u32))>> for QuotedAssets {
    fn from(pairs: Vec<(String, (u128, u32))>) -> Self {
        let quoted_assets = Self::default();
        for (ticker, (price, decimals)) in pairs {
            let decimal_price = Decimal::new(price as i64, decimals);
            if !decimal_price.is_sign_negative() {
                quoted_assets.0.insert(ticker.to_uppercase(), decimal_price);
            }
        }
        quoted_assets
    }
}
