use pragma_db::models::IndexerState;
use pragma_quoting::currencies::{CURRENCIES_PRICES, Currency};
use rust_decimal::Decimal;

use crate::errors::ApiError;

pub fn map_status(status: &str) -> String {
    match status {
        "active" => "live".to_string(),
        other => other.to_string(),
    }
}

/// Vaults 2 through 6 rely on the alternative vault API
pub fn is_alternative_vault(vault_id: &str) -> bool {
    vault_id
        .parse::<i32>()
        .map(|id| (2..=6).contains(&id))
        .unwrap_or(false)
}

/// Check if the indexer is ready to serve data for a vault
/// Returns an error if the indexer is not synced or has errors
pub async fn validate_indexer_status(
    vault_id: &str,
    pool: &deadpool_diesel::postgres::Pool,
) -> Result<(), ApiError> {
    let conn = pool.get().await.map_err(|e| {
        tracing::error!(
            "Failed to get database connection for indexer status check: {}",
            e
        );
        ApiError::InternalServerError
    })?;

    let vault_id = vault_id.to_string();
    let indexer_state = conn
        .interact(move |conn| IndexerState::find_by_vault_id(&vault_id, conn))
        .await
        .map_err(|e| {
            tracing::error!("Database interaction error for indexer status: {}", e);
            ApiError::InternalServerError
        })?
        .map_err(|e| {
            tracing::error!("Failed to fetch indexer state: {}", e);
            ApiError::InternalServerError
        })?;

    // Check if indexer has errors
    if indexer_state.is_error() {
        return Err(ApiError::ServiceUnavailable(
            "Indexer is currently experiencing issues. Please try again later.".to_string(),
        ));
    }

    // Check if indexer is synced
    if !indexer_state.is_synced() {
        return Err(ApiError::ServiceUnavailable(
            "Indexer is still syncing. Data may be incomplete. Please try again later.".to_string(),
        ));
    }

    Ok(())
}

/// Quote an amount to a target currencys
pub async fn quote_to_currency(
    amount: Decimal,
    target_currency: Currency,
) -> Result<Decimal, ApiError> {
    // Get the price of the target currency in USD
    let price = CURRENCIES_PRICES.of(target_currency).await.map_err(|e| {
        tracing::error!("Failed to fetch price for {:?}: {}", target_currency, e);
        ApiError::InternalServerError
    })?;

    Ok(amount / price)
}
