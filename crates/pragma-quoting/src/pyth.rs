use anyhow::anyhow;
use rust_decimal::Decimal;
use serde::Deserialize;

use crate::currencies::Currency;

// Pyth response structures
#[derive(Debug, Deserialize)]
struct PythResponse {
    parsed: Vec<PriceFeed>,
}

#[derive(Debug, Deserialize)]
struct PriceFeed {
    id: String,
    price: PriceData,
}

#[derive(Debug, Deserialize)]
struct PriceData {
    price: String,
    expo: i32,
}

/// Fetch the current price from Pyth for a given currency
pub async fn fetch_pyth_price(currency: Currency) -> anyhow::Result<Decimal> {
    if currency == Currency::USD {
        return Ok(Decimal::ONE);
    }

    let feed_id = get_feed_id(currency);

    let url = format!("https://hermes.pyth.network/v2/updates/price/latest?ids[]={feed_id}");

    let client = reqwest::Client::new();
    let response = client.get(&url).send().await?.error_for_status()?;

    let pyth_response: PythResponse = response.json().await?;

    let price_feed = pyth_response
        .parsed
        .iter()
        .find(|feed| feed.id.replace("0x", "") == feed_id.replace("0x", ""))
        .ok_or_else(|| anyhow!("No price feed found for {:?}", currency))?;

    let price_int: i64 = price_feed.price.price.parse()?;
    let expo = price_feed.price.expo;

    let divisor = 10_i64.pow(expo.unsigned_abs());
    let price = Decimal::from(price_int) / Decimal::from(divisor);

    Ok(price)
}

/// Get the Pyth feed ID for a given currency (all against USD)
fn get_feed_id(currency: Currency) -> &'static str {
    match currency {
        Currency::USDC => "0xeaa020c61cc479712813461ce153894a96a6c00b21ed0cfc2798d1f9a9e9c94a",
        Currency::USD => unreachable!("Already handled"),
    }
}
