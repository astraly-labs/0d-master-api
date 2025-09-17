use std::collections::HashSet;

use chrono::{DateTime, Utc};
use deadpool_diesel::postgres::Pool;
use evian::contracts::starknet::vault::data::indexer::events::{
    DepositEvent, RedeemRequestedEvent, VaultAddress, VaultEvent,
};
use evian::utils::indexer::handler::StarknetBlockHeader;
use evian::{
    contracts::starknet::vault::StarknetVaultIndexer, utils::indexer::handler::OutputEvent,
};
use pragma_db::models;
use starknet::core::types::Felt;
use task_supervisor::{SupervisedTask, TaskError};

#[derive(Clone)]
pub struct ExtendedVault {
    pub current_block: u64,
    pub apibara_api_key: String,
    pub vault_address: Felt,
    pub db_pool: Pool,
}

#[async_trait::async_trait]
impl SupervisedTask for ExtendedVault {
    async fn run(&mut self) -> Result<(), TaskError> {
        let vault_indexer = StarknetVaultIndexer::new(
            self.apibara_api_key.clone(),
            HashSet::from([VaultAddress(self.vault_address)]),
            self.current_block,
        );

        let (mut event_receiver, mut vault_handle) = vault_indexer.start().await?;
        tracing::info!(
            "[Indexer] 🔌 Connected to the on-chain Extended-Vault! (from block {})",
            self.current_block
        );

        loop {
            tokio::select! {
                Some(output_event) = event_receiver.recv() => {
                    match output_event {
                        OutputEvent::Event { header, event } => {
                            self.handle_event(header, event).await?;
                        }
                        OutputEvent::Synced => {
                            todo!()
                        }
                        // NOTE: Never happens for now. See later when apibara upgrades?
                        OutputEvent::Finalized(_) | OutputEvent::Invalidated(_) => { }
                    }
                }
                res = &mut vault_handle => {
                    anyhow::bail!("😱 Vault indexer stopped: {res:?}");
                }
            }
        }
    }
}

impl ExtendedVault {
    async fn handle_event(
        &mut self,
        header: Option<StarknetBlockHeader>,
        event: VaultEvent,
    ) -> Result<(), anyhow::Error> {
        // Extract block information for persistence
        let (block_number, block_timestamp) = if let Some(h) = header {
            (h.block_number, h.timestamp)
        } else {
            todo!()
        };

        match event {
            VaultEvent::Report(_) => {
                // Report events don't directly create user transactions, but they provide important
                // vault state information that could be used for calculating share prices
            }
            VaultEvent::RedeemRequested(redeem) => {
                self.handle_redeem_requested_event(redeem, block_number, block_timestamp)
                    .await?
            }
            VaultEvent::BringLiquidity(_) => {
                // BringLiquidity events represent internal vault operations (rebalancing)
                // These don't directly affect user positions but provide valuable vault state info
            }
            VaultEvent::Deposit(deposit) => {
                self.handle_deposit_event(deposit, block_number, block_timestamp)
                    .await?
            }
        }

        Ok(())
    }

    async fn handle_redeem_requested_event(
        &self,
        redeem: RedeemRequestedEvent,
        block_number: u64,
        block_timestamp: i64,
    ) -> Result<(), anyhow::Error> {
        todo!();
    }

    async fn handle_deposit_event(
        &self,
        deposit: DepositEvent,
        block_number: u64,
        block_timestamp: i64,
    ) -> Result<(), anyhow::Error> {
        todo!();
    }
}
