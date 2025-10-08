use std::{future::Future, sync::Arc};

use rust_decimal::Decimal;
use tiny_keccak::Hasher;
use zerod_db::{
    ZerodPool,
    models::{IndexerState, Vault},
};
use zerod_master::{JaffarClient, MasterApiError, VaultMasterClient, VesuClient};
use zerod_quoting::currencies::{CURRENCIES_PRICES, Currency};

use crate::{
    AppState,
    errors::{ApiError, DatabaseErrorExt},
};

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

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum VaultBackend {
    Jaffar,
    Vesu,
}

impl VaultBackend {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Jaffar => "jaffar",
            Self::Vesu => "vesu",
        }
    }

    pub fn from_vault(vault: &Vault) -> Self {
        if is_alternative_vault(&vault.id) {
            Self::Vesu
        } else {
            Self::Jaffar
        }
    }
}

pub struct VaultBackendClient {
    backend: VaultBackend,
    client: Arc<dyn VaultMasterClient + Send + Sync>,
}

impl VaultBackendClient {
    pub fn new(vault: &Vault) -> Result<Self, ApiError> {
        let backend = VaultBackend::from_vault(vault);
        let client: Arc<dyn VaultMasterClient + Send + Sync> = match backend {
            VaultBackend::Jaffar => Arc::new(JaffarClient::new(&vault.api_endpoint)),
            VaultBackend::Vesu => {
                let client = VesuClient::new(&vault.api_endpoint, &vault.contract_address)
                    .map_err(|err| {
                        tracing::error!(
                            vault_id = %vault.id,
                            error = %err,
                            "Failed to create Vesu client",
                        );
                        ApiError::InternalServerError
                    })?;
                Arc::new(client)
            }
        };

        Ok(Self { backend, client })
    }

    pub const fn backend(&self) -> VaultBackend {
        self.backend
    }

    pub fn client(&self) -> Arc<dyn VaultMasterClient + Send + Sync> {
        Arc::clone(&self.client)
    }
}

pub async fn fetch_vault(state: &AppState, vault_id: &str) -> Result<Vault, ApiError> {
    let vault_id_owned = vault_id.to_string();
    state
        .pool
        .interact_with_context(format!("find vault by id: {vault_id}"), move |conn| {
            Vault::find_by_id(&vault_id_owned, conn)
        })
        .await
        .map_err(|e| e.or_not_found(format!("Vault {vault_id} not found")))
}

pub async fn fetch_vault_with_client(
    state: &AppState,
    vault_id: &str,
) -> Result<(Vault, VaultBackendClient), ApiError> {
    let vault = fetch_vault(state, vault_id).await?;
    let client = VaultBackendClient::new(&vault)?;
    Ok((vault, client))
}

pub async fn call_vault_backend<T, F, Fut>(
    client: &VaultBackendClient,
    vault: &Vault,
    operation: &'static str,
    f: F,
) -> Result<T, ApiError>
where
    F: FnOnce(Arc<dyn VaultMasterClient + Send + Sync>) -> Fut,
    Fut: Future<Output = Result<T, MasterApiError>> + Send,
{
    let backend = client.backend();
    let client_arc = client.client();

    f(client_arc).await.map_err(|err| {
        tracing::error!(
            vault_id = %vault.id,
            backend = backend.as_str(),
            operation,
            error = %err,
            "Vault backend call failed",
        );
        ApiError::InternalServerError
    })
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

pub fn normalize_starknet_address(address: &str) -> Result<String, ApiError> {
    let trimmed = address.trim();
    let without_prefix = trimmed
        .strip_prefix("0x")
        .or_else(|| trimmed.strip_prefix("0X"))
        .unwrap_or(trimmed);

    if without_prefix.is_empty() || without_prefix.len() > 64 {
        return Err(ApiError::BadRequest(
            "Receiver address must be a hexadecimal string up to 32 bytes".to_string(),
        ));
    }

    if !without_prefix.chars().all(|c| c.is_ascii_hexdigit()) {
        return Err(ApiError::BadRequest(
            "Receiver address must be a hexadecimal string".to_string(),
        ));
    }

    let padded = format!("{:0>64}", without_prefix.to_lowercase());
    let address_bytes = hex::decode(&padded).map_err(|_| {
        ApiError::BadRequest("Receiver address must be a valid hexadecimal string".to_string())
    })?;

    let hash = starknet_keccak_bytes(&address_bytes);
    let hash_hex = hex::encode(hash);

    let mut checksummed = String::with_capacity(66);
    checksummed.push_str("0x");

    for (idx, ch) in padded.chars().enumerate() {
        if ch.is_ascii_digit() {
            checksummed.push(ch);
            continue;
        }

        let hash_char = hash_hex.as_bytes()[idx];
        if hash_char >= b'8' {
            checksummed.push(ch.to_ascii_uppercase());
        } else {
            checksummed.push(ch);
        }
    }

    Ok(checksummed)
}

fn starknet_keccak_bytes(data: &[u8]) -> [u8; 32] {
    let mut hasher = tiny_keccak::Keccak::v256();
    hasher.update(data);
    let mut hash = [0u8; 32];
    hasher.finalize(&mut hash);
    hash[0] &= 0b0000_0011;
    hash
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
