use rust_decimal::Decimal;
use zerod_db::{ZerodPool, models::IndexerState};
use zerod_quoting::currencies::{CURRENCIES_PRICES, Currency};

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
    let vault_id_clone = vault_id.to_string();
    let indexer_state = pool
        .interact_with_context(
            format!("check indexer status for vault: {vault_id}"),
            move |conn| IndexerState::find_by_vault_id(&vault_id_clone, conn),
        )
        .await?;

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
        tracing::error!("Failed to fetch price for {target_currency:?}: {e}");
        ApiError::InternalServerError
    })?;

    Ok(amount / price)
}
