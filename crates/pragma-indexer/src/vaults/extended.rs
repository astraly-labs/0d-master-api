use std::collections::HashSet;

use chrono::{DateTime, Utc};
use evian::contracts::starknet::vault::data::indexer::events::{
    DepositEvent, RedeemRequestedEvent, VaultAddress, VaultEvent,
};
use evian::{
    contracts::starknet::vault::StarknetVaultIndexer, utils::indexer::handler::OutputEvent,
};
use pragma_db::models::{
    user::User,
    user_position::{NewUserPosition, UserPosition, UserPositionUpdate},
    user_transaction::{NewUserTransaction, TransactionStatus, TransactionType, UserTransaction},
    vault::Vault,
};
use rust_decimal::Decimal;
use starknet::core::types::Felt;
use task_supervisor::{SupervisedTask, TaskError};

use crate::vaults::helpers::felt_to_hex_str;
use crate::vaults::state::VaultState;

#[derive(Clone)]
pub struct ExtendedVault {
    pub apibara_api_key: String,
    pub vault_address: Felt,
    pub vault_id: String,
    pub state: VaultState,
}

#[async_trait::async_trait]
impl SupervisedTask for ExtendedVault {
    async fn run(&mut self) -> Result<(), TaskError> {
        // Validate that the vault exists before starting
        self.vault_exists().await?;

        // Load the last processed block from the database
        self.state.load_last_processed_block(&self.vault_id).await?;

        let vault_indexer = StarknetVaultIndexer::new(
            self.apibara_api_key.clone(),
            HashSet::from([VaultAddress(self.vault_address)]),
            self.state.current_block,
        );

        // Initialize indexer state with starting block
        self.state.initialize_indexer_state(&self.vault_id).await?;

        let (mut event_receiver, mut vault_handle) = vault_indexer.start().await?;
        tracing::info!(
            "[ExtendedVault] 🔌 Connected to the on-chain Vault! (from block {})",
            self.state.current_block
        );

        loop {
            tokio::select! {
                Some(output_event) = event_receiver.recv() => {
                    match output_event {
                        OutputEvent::Event { header, event, tx_hash } => {
                            let (block_number, block_timestamp) =
                            header.map_or_else(|| todo!(), |h| (h.block_number, h.timestamp));

                            let block_timestamp = DateTime::from_timestamp_secs(block_timestamp).unwrap_or_else(|| {
                                panic!("[ExtendedVault] ❌ Invalid timestamp for block {block_number}")
                            });

                            let tx_hash = tx_hash.map(felt_to_hex_str).expect("[ExtendedVault] ❌ Invalid transaction hash");

                            if let Err(e) = self.handle_event(block_number, block_timestamp, event, tx_hash).await {
                                self.state.record_indexer_state_error(&self.vault_id, e.to_string()).await?;
                                return Err(TaskError::from(e));
                            }

                            // Update indexer state
                            self.state.current_block = block_number;
                            self.state.current_timestamp = Some(block_timestamp);
                            self.state.update_indexer_state(&self.vault_id, block_number, block_timestamp).await?;
                        }
                        OutputEvent::Synced => {
                            tracing::info!("[ExtendedVault] 🥳 Indexer reached the tip of the chain!");
                            // TODO: Should we flag as synced?
                        }
                        // NOTE: Never happens for now. See later when apibara upgrades?
                        OutputEvent::Finalized(_) | OutputEvent::Invalidated(_) => { }
                    }
                }
                res = &mut vault_handle => {
                    let error_msg = format!("😱 Vault indexer stopped: {res:?}");
                    self.state.record_indexer_state_error(&self.vault_id, error_msg.clone()).await?;
                    anyhow::bail!("{error_msg}");
                }
            }
        }
    }
}

impl ExtendedVault {
    async fn handle_event(
        &self,
        block_number: u64,
        block_timestamp: DateTime<Utc>,
        event: VaultEvent,
        tx_hash: String,
    ) -> Result<(), anyhow::Error> {
        match event {
            VaultEvent::Deposit(deposit) => {
                self.handle_deposit_event(deposit, tx_hash, block_number, block_timestamp)
                    .await?;
            }
            VaultEvent::RedeemRequested(redeem) => {
                self.handle_redeem_requested_event(redeem, tx_hash, block_number, block_timestamp)
                    .await?;
            }
            #[allow(clippy::match_same_arms)]
            VaultEvent::Report(_) => {
                // Report events don't directly create user transactions, but they provide important
                // vault state information that could be used for calculating share prices
            }
            VaultEvent::BringLiquidity(_) => {
                // BringLiquidity events represent internal vault operations (rebalancing)
                // These don't directly affect user positions but provide valuable vault state info
            }
        }

        Ok(())
    }

    async fn handle_deposit_event(
        &self,
        deposit: DepositEvent,
        tx_hash: String,
        block_number: u64,
        block_timestamp: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        //tracing::info!("[ExtendedVault] 💰 Handling deposit event with hash: {tx_hash}");

        let user_address = felt_to_hex_str(deposit.owner);
        self.ensure_user_exists(user_address.clone()).await?;

        // NOTE: This is the share price at the time of the deposit
        let share_price = if deposit.shares > Decimal::ZERO {
            Some(deposit.assets / deposit.shares)
        } else {
            None
        };

        tracing::info!(
            "[ExtendedVault] 💰 Deposit event with hash: {tx_hash} [share_price: {share_price:?}]"
        );

        // Check if transaction already exists to avoid duplicates
        let conn = self.state.db_pool.get().await?;
        let tx_exists = conn
            .interact({
                let tx_hash_check = tx_hash.clone();
                move |conn| UserTransaction::exists_by_hash(&tx_hash_check, conn)
            })
            .await
            .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Database interaction failed: {e}"))?
            .map_err(anyhow::Error::from)?;

        if tx_exists {
            tracing::info!(
                "[ExtendedVault] ⏭️  Skipping duplicate deposit transaction: {} (block: {})",
                tx_hash,
                block_number
            );
            return Ok(());
        }

        let new_transaction = NewUserTransaction {
            tx_hash,
            block_number: block_number
                .try_into()
                .expect("[ExtendedVault] 🌯 Block number too large for i64"),
            block_timestamp,
            user_address: user_address.clone(),
            vault_id: self.vault_id.clone(),
            type_: TransactionType::Deposit.as_str().to_string(),
            status: TransactionStatus::Confirmed.as_str().to_string(),
            amount: deposit.assets,
            shares_amount: Some(deposit.shares),
            share_price,
            gas_fee: None,
            metadata: None,
        };

        let conn = self.state.db_pool.get().await?;
        conn.interact(move |conn| UserTransaction::create(&new_transaction, conn))
            .await
            .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Transaction creation failed: {e}"))?
            .map_err(anyhow::Error::from)?;

        let conn = self.state.db_pool.get().await?;
        let vault_id = self.vault_id.clone();
        conn.interact(move |conn| {
            match UserPosition::find_by_user_and_vault(&user_address, &vault_id, conn) {
                Ok(position) => {
                    // Add deposit to existing position
                    let new_share_balance = position.share_balance + deposit.shares;
                    let new_cost_basis = position.cost_basis + deposit.assets;

                    let updates = UserPositionUpdate {
                        share_balance: Some(new_share_balance),
                        cost_basis: Some(new_cost_basis),
                        last_activity_at: Some(block_timestamp),
                        updated_at: Some(Utc::now()),
                    };

                    position.update(&updates, conn)
                }
                Err(diesel::result::Error::NotFound) => {
                    // Create new position for the deposit
                    let new_position = NewUserPosition {
                        user_address,
                        vault_id: vault_id.clone(),
                        share_balance: deposit.shares,
                        cost_basis: deposit.assets,
                        first_deposit_at: Some(block_timestamp),
                        last_activity_at: Some(block_timestamp),
                    };

                    UserPosition::create(&new_position, conn)
                }
                Err(e) => Err(e),
            }
        })
        .await
        .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Position update failed: {e}"))?
        .map_err(anyhow::Error::from)?;

        Ok(())
    }
    async fn handle_redeem_requested_event(
        &self,
        redeem: RedeemRequestedEvent,
        tx_hash: String,
        block_number: u64,
        block_timestamp: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        tracing::info!("[ExtendedVault] 💸 Handling redeem requested event with hash: {tx_hash}");

        let user_address = felt_to_hex_str(redeem.owner);
        self.ensure_user_exists(user_address.clone()).await?;

        // NOTE: This is the share price at the time of the redeem request
        let share_price = if redeem.shares > Decimal::ZERO {
            Some(redeem.assets / redeem.shares)
        } else {
            None
        };

        // Check if transaction already exists to avoid duplicates
        let conn = self.state.db_pool.get().await?;
        let tx_exists = conn
            .interact({
                let tx_hash_check = tx_hash.clone();
                move |conn| UserTransaction::exists_by_hash(&tx_hash_check, conn)
            })
            .await
            .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Database interaction failed: {e}"))?
            .map_err(anyhow::Error::from)?;

        if tx_exists {
            tracing::info!(
                "[ExtendedVault] ⏭️  Skipping duplicate withdraw transaction: {} (block: {})",
                tx_hash,
                block_number
            );
            return Ok(());
        }

        // Create transaction record for withdrawal
        let new_transaction = NewUserTransaction {
            tx_hash,
            block_number: block_number
                .try_into()
                .expect("[ExtendedVault] 🌯 Block number too large for i64"),
            block_timestamp,
            user_address: user_address.clone(),
            vault_id: self.vault_id.clone(),
            type_: TransactionType::Withdraw.as_str().to_string(),
            status: TransactionStatus::Confirmed.as_str().to_string(), // TODO: Change to Confirmed only when the redeem is completed?
            amount: redeem.assets,
            shares_amount: Some(redeem.shares),
            share_price,
            gas_fee: None,
            metadata: Some(serde_json::json!({
                "redeem_id": redeem.id.to_string(),
                "epoch": redeem.epoch.to_string(),
                "receiver": redeem.receiver.to_string()
            })),
        };

        // First database operation: Create transaction record
        let conn = self.state.db_pool.get().await?;
        conn.interact(move |conn| UserTransaction::create(&new_transaction, conn))
            .await
            .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Transaction creation failed: {e}"))?
            .map_err(anyhow::Error::from)?;

        // Second database operation: Update user position
        let conn = self.state.db_pool.get().await?;
        let vault_id = self.vault_id.clone();
        conn.interact(move |conn| {
            match UserPosition::find_by_user_and_vault(&user_address, &vault_id, conn) {
                Ok(position) => {
                    // Reduce share balance for pending redemption
                    let new_share_balance = position.share_balance - redeem.shares;

                    // Ensure we don't go negative
                    let new_share_balance = if new_share_balance < Decimal::ZERO {
                        Decimal::ZERO
                    } else {
                        new_share_balance
                    };

                    let updates = UserPositionUpdate {
                        share_balance: Some(new_share_balance),
                        cost_basis: None, // TODO: Update the cost basis when redeem is completed?
                        last_activity_at: Some(block_timestamp),
                        updated_at: Some(Utc::now()),
                    };

                    position.update(&updates, conn).map(|_| ())
                }
                Err(diesel::result::Error::NotFound) => {
                    tracing::warn!(
                        "Redeem requested for non-existent position: user={}, vault={}",
                        user_address,
                        vault_id
                    );
                    Ok(())
                }
                Err(e) => Err(e),
            }
        })
        .await
        .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Position update failed: {e}"))?
        .map_err(anyhow::Error::from)?;

        Ok(())
    }

    /// Validate that the vault exists in the database before starting indexer
    async fn vault_exists(&self) -> Result<(), anyhow::Error> {
        let vault_id = self.vault_id.clone();
        let conn = self.state.db_pool.get().await?;

        let result = conn
            .interact(move |conn| Vault::find_by_id(&vault_id, conn))
            .await
            .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Database interaction failed: {e}"))?;

        match result {
            Ok(vault) => {
                tracing::info!(
                    "[ExtendedVault] 🔎 Vault exists in database with name({}) and id({})",
                    vault.name,
                    vault.id
                );
                Ok(())
            }
            Err(diesel::result::Error::NotFound) => {
                anyhow::bail!(
                    "[ExtendedVault] ❌ Vault with ID '{}' not found in database.\n\
                    To fix this error, create the vault record in the database.",
                    self.vault_id
                );
            }
            Err(e) => {
                anyhow::bail!("[ExtendedVault] 🗃️ Database error while checking vault: {e}");
            }
        }
    }

    /// Ensure user exists in database
    async fn ensure_user_exists(&self, user_address: String) -> Result<(), anyhow::Error> {
        let conn = self.state.db_pool.get().await?;

        conn.interact(move |conn| {
            User::find_or_create(&user_address, pragma_common::web3::Chain::Starknet, conn)
        })
        .await
        .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Database interaction failed: {e}"))?
        .map_err(anyhow::Error::from)?;

        Ok(())
    }
}
