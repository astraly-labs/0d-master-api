use std::{collections::HashMap, str::FromStr, sync::Arc};

use anyhow::{Context, Result, bail};
use async_trait::async_trait;
use num_bigint::BigUint;
use rust_decimal::Decimal;
use starknet::core::types::{
    BlockId, BlockTag, Felt, MaybePreConfirmedBlockWithTxHashes, TransactionReceipt, U256,
};
use starknet::providers::Provider;
use starknet::signers::SigningKey;
use starknet_crypto::Felt as SigningFelt;
use tokio::sync::Mutex;

use pragma_common::starknet::FallbackProvider;

use crate::config::StarknetAccountConfig;

#[async_trait]
pub trait StarknetClient: Send + Sync {
    async fn get_report_delay(&self, contract_address: Felt) -> Result<u64>;
    async fn get_last_report_timestamp(&self, contract_address: Felt) -> Result<u64>;
    async fn get_current_block_timestamp(&self) -> Result<u64>;
    async fn get_current_block_number(&self) -> Result<u64>;
    async fn get_buffer(&self, contract_address: Felt) -> Result<Decimal>;
    async fn get_aum(&self, contract_address: Felt) -> Result<Decimal>;
    async fn get_handled_epoch_len(&self, contract_address: Felt) -> Result<Decimal>;
    async fn trigger_report(&self, contract_address: Felt, new_aum: Decimal) -> Result<String>;
    async fn claim_redeems(&self, contract_address: Felt, redeem_ids: Vec<U256>) -> Result<String>;
}

#[derive(Clone)]
pub struct EvianStarknetClient {
    provider: FallbackProvider,
    account_address: Felt,
    signing_key: SigningFelt,
    contracts: Arc<
        Mutex<
            HashMap<
                String,
                Arc<evian::contracts::starknet::vault::StarknetVaultContract<FallbackProvider>>,
            >,
        >,
    >,
}

impl EvianStarknetClient {
    pub fn new(provider: FallbackProvider, credentials: &StarknetAccountConfig) -> Self {
        Self {
            provider,
            account_address: credentials.account_address,
            signing_key: credentials.private_key,
            contracts: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    async fn get_contract(
        &self,
        contract_address: Felt,
    ) -> Result<Arc<evian::contracts::starknet::vault::StarknetVaultContract<FallbackProvider>>>
    {
        let key = format!("{contract_address:#x}");
        {
            let cache = self.contracts.lock().await;
            if let Some(contract) = cache.get(&key) {
                return Ok(contract.clone());
            }
        }

        let contract = Arc::new(
            evian::contracts::starknet::vault::StarknetVaultContract::new_with_account(
                self.provider.clone(),
                contract_address,
                self.account_address,
                SigningKey::from_secret_scalar(self.signing_key),
            )
            .await
            .context("Failed to initialize Starknet vault contract")?,
        );

        let mut cache = self.contracts.lock().await;
        let entry = cache.entry(key).or_insert_with(|| contract.clone());

        Ok(entry.clone())
    }

    fn get_contract_read_only(
        &self,
        contract_address: Felt,
    ) -> Arc<evian::contracts::starknet::vault::StarknetVaultContract<FallbackProvider>> {
        Arc::new(
            evian::contracts::starknet::vault::StarknetVaultContract::new(
                self.provider.clone(),
                contract_address,
            ),
        )
    }

    fn decimal_to_u256(value: Decimal) -> Result<U256> {
        let scaled = value.trunc();
        if scaled != value {
            bail!("AUM value contains fractional component: {value}");
        }

        let big = BigUint::from_str(&scaled.to_string())
            .context("Failed to convert decimal to BigUint")?;
        let bytes = big.to_bytes_be();
        if bytes.len() > 32 {
            bail!("Value {value} does not fit into 256 bits");
        }

        let mut padded = [0u8; 32];
        let start = 32 - bytes.len();
        padded[start..].copy_from_slice(&bytes);

        let high = u128::from_be_bytes(padded[..16].try_into().expect("slice length checked"));
        let low = u128::from_be_bytes(padded[16..].try_into().expect("slice length checked"));
        Ok(U256::from_words(low, high))
    }

    fn receipt_to_hash(receipt: &TransactionReceipt) -> String {
        format!("{:#x}", receipt.transaction_hash())
    }
}

#[async_trait]
impl StarknetClient for EvianStarknetClient {
    async fn get_report_delay(&self, contract_address: Felt) -> Result<u64> {
        let contract = self.get_contract_read_only(contract_address);
        contract
            .report_delay(None)
            .await
            .context("Fetching report delay failed")
    }

    async fn get_last_report_timestamp(&self, contract_address: Felt) -> Result<u64> {
        let contract = self.get_contract_read_only(contract_address);
        contract
            .last_report_timestamp(None)
            .await
            .context("Fetching last report timestamp failed")
    }

    async fn get_current_block_timestamp(&self) -> Result<u64> {
        let block = self
            .provider
            .get_block_with_tx_hashes(BlockId::Tag(BlockTag::Latest))
            .await
            .context("Failed to fetch latest Starknet block")?;

        let timestamp = match block {
            MaybePreConfirmedBlockWithTxHashes::Block(block) => block.timestamp,
            MaybePreConfirmedBlockWithTxHashes::PreConfirmedBlock(block) => block.timestamp,
        };

        Ok(timestamp)
    }

    async fn get_current_block_number(&self) -> Result<u64> {
        self.provider
            .block_number()
            .await
            .context("Failed to fetch latest block number")
    }

    async fn get_buffer(&self, contract_address: Felt) -> Result<Decimal> {
        let contract = self.get_contract_read_only(contract_address);
        contract
            .buffer(None)
            .await
            .context("Fetching vault buffer failed")
    }

    async fn get_aum(&self, contract_address: Felt) -> Result<Decimal> {
        let contract = self.get_contract_read_only(contract_address);
        contract
            .aum(None)
            .await
            .context("Fetching vault AUM failed")
    }

    async fn get_handled_epoch_len(&self, contract_address: Felt) -> Result<Decimal> {
        let contract = self.get_contract_read_only(contract_address);
        contract
            .handled_epochs(None)
            .await
            .context("Fetching handled epochs failed")
    }

    async fn trigger_report(&self, contract_address: Felt, new_aum: Decimal) -> Result<String> {
        let u256_aum = Self::decimal_to_u256(new_aum)?;
        let contract = self.get_contract(contract_address).await?;
        let receipt = contract
            .report(u256_aum)
            .await
            .context("Failed to send report transaction")?;
        Ok(Self::receipt_to_hash(&receipt))
    }

    async fn claim_redeems(&self, contract_address: Felt, redeem_ids: Vec<U256>) -> Result<String> {
        let contract = self.get_contract(contract_address).await?;
        let mut last_hash = None;

        for redeem_id in redeem_ids {
            let receipt = contract
                .claim_redeem(redeem_id)
                .await
                .context("Failed to claim redeem on Starknet")?;
            last_hash = Some(Self::receipt_to_hash(&receipt));
        }

        Ok(last_hash.unwrap_or_default())
    }
}
