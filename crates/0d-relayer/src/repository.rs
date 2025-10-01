use std::{collections::HashSet, str::FromStr};

use anyhow::{Context, Result, anyhow, bail};
use deadpool_diesel::postgres::Pool;
use num_bigint::BigUint;
use pragma_db::models::user_transaction::{TransactionStatus, TransactionType};
use pragma_db::models::{IndexerState, UserTransaction, Vault};
use rust_decimal::Decimal;
use serde::Deserialize;
use starknet::core::types::{Felt, U256};

#[derive(Debug, Clone)]
pub struct VaultRecord {
    pub id: String,
    pub contract_address: Felt,
    pub aum_provider: Option<String>,
}

#[derive(Debug, Clone)]
pub struct IndexerStatusRecord {
    pub last_processed_block: i64,
}

#[derive(Debug, Clone)]
pub struct PendingRedeem {
    pub redeem_id: U256,
    pub epoch: Option<Decimal>,
    pub user_address: String,
}

pub struct RelayerRepository {
    pool: Pool,
}

impl RelayerRepository {
    pub const fn new(pool: Pool) -> Self {
        Self { pool }
    }

    pub async fn fetch_live_vaults(&self) -> Result<Vec<VaultRecord>> {
        let pool = self.pool.clone();
        let vaults = pool
            .get()
            .await
            .context("Failed to acquire database connection")?
            .interact(|conn| Vault::find_live(conn))
            .await
            .map_err(|err| anyhow!("Database interaction for live vaults failed: {err}"))??;

        vaults
            .into_iter()
            .map(|vault| {
                let contract_address = Felt::from_hex(&vault.contract_address)
                    .with_context(|| format!("Invalid Starknet address for vault {}", vault.id))?;

                Ok(VaultRecord {
                    id: vault.id,
                    contract_address,
                    aum_provider: None,
                })
            })
            .collect()
    }

    pub async fn fetch_indexer_state(&self, vault_id: &str) -> Result<Option<IndexerStatusRecord>> {
        let pool = self.pool.clone();
        let vault = vault_id.to_string();
        let maybe_state = pool
            .get()
            .await
            .context("Failed to acquire database connection")?
            .interact(move |conn| IndexerState::find_by_vault_id(&vault, conn))
            .await
            .map_err(|err| anyhow!("Database interaction for indexer_state failed: {err}"));

        match maybe_state {
            Ok(Ok(state)) => Ok(Some(IndexerStatusRecord {
                last_processed_block: state.last_processed_block,
            })),
            Ok(Err(diesel::result::Error::NotFound)) => Ok(None),
            Ok(Err(err)) => Err(err.into()),
            Err(err) => Err(err),
        }
    }

    pub async fn fetch_pending_redeems_below_epoch(
        &self,
        vault_id: &str,
        handled_epoch_len: Decimal,
    ) -> Result<Vec<PendingRedeem>> {
        let pool = self.pool.clone();
        let vault = vault_id.to_string();

        let transactions = pool
            .get()
            .await
            .context("Failed to acquire database connection")?
            .interact({
                let vault = vault.clone();
                move |conn| {
                    use diesel::prelude::*;
                    use pragma_db::schema::user_transactions::dsl;

                    dsl::user_transactions
                        .filter(dsl::vault_id.eq(&vault))
                        .filter(dsl::type_.eq(TransactionType::Withdraw.as_str()))
                        .filter(dsl::status.eq(TransactionStatus::Pending.as_str()))
                        .order(dsl::block_timestamp.asc())
                        .load::<UserTransaction>(conn)
                }
            })
            .await
            .map_err(|err| anyhow!("Database interaction for pending redeems failed: {err}"))??;

        let confirmed_transactions = pool
            .get()
            .await
            .context("Failed to acquire database connection")?
            .interact({
                let vault = vault.clone();
                move |conn| {
                    use diesel::prelude::*;
                    use pragma_db::schema::user_transactions::dsl;

                    dsl::user_transactions
                        .filter(dsl::vault_id.eq(&vault))
                        .filter(dsl::type_.eq(TransactionType::Withdraw.as_str()))
                        .filter(dsl::status.eq(TransactionStatus::Confirmed.as_str()))
                        .load::<UserTransaction>(conn)
                }
            })
            .await
            .map_err(|err| anyhow!("Database interaction for confirmed redeems failed: {err}"))??;

        let claimed_ids: HashSet<U256> = confirmed_transactions
            .into_iter()
            .filter_map(|tx| parse_redeem_metadata(&tx).map(|(redeem_id, _)| redeem_id))
            .collect();

        let mut redeems = Vec::with_capacity(transactions.len());

        for tx in transactions {
            let Some((redeem_id_u256, epoch_opt)) = parse_redeem_metadata(&tx) else {
                continue;
            };

            if claimed_ids.contains(&redeem_id_u256) {
                continue;
            }

            if let Some(epoch_value) = epoch_opt {
                if epoch_value >= handled_epoch_len {
                    continue;
                }
            }

            redeems.push(PendingRedeem {
                redeem_id: redeem_id_u256,
                epoch: epoch_opt,
                user_address: tx.user_address,
            });
        }

        Ok(redeems)
    }
}

impl Clone for RelayerRepository {
    fn clone(&self) -> Self {
        Self {
            pool: self.pool.clone(),
        }
    }
}

#[derive(Debug, Deserialize)]
struct RedeemMetadata {
    #[serde(default)]
    redeem_id: Option<String>,
    #[serde(default)]
    epoch: Option<String>,
}

fn u256_from_decimal_str(value: &str) -> Result<U256> {
    let big_uint = BigUint::from_str(value)
        .with_context(|| format!("Invalid decimal string for U256: {value}"))?;
    let bytes = big_uint.to_bytes_be();
    if bytes.len() > 32 {
        bail!("Value {value} does not fit into 256 bits");
    }

    if bytes.is_empty() {
        return Ok(U256::from_words(0, 0));
    }

    let mut padded = [0u8; 32];
    let start = 32 - bytes.len();
    padded[start..].copy_from_slice(&bytes);

    let high = u128::from_be_bytes(padded[..16].try_into().expect("slice length checked"));
    let low = u128::from_be_bytes(padded[16..].try_into().expect("slice length checked"));
    Ok(U256::from_words(low, high))
}

fn parse_redeem_metadata(tx: &UserTransaction) -> Option<(U256, Option<Decimal>)> {
    let metadata = tx.metadata.as_ref()?;
    let parsed: RedeemMetadata = serde_json::from_value(metadata.clone()).ok()?;

    let redeem_id = parsed
        .redeem_id
        .as_deref()
        .and_then(|value| u256_from_decimal_str(value).ok())?;

    let epoch = parsed
        .epoch
        .as_deref()
        .and_then(|value| Decimal::from_str(value).ok());

    Some((redeem_id, epoch))
}
