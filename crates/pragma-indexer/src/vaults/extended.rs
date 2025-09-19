use std::collections::HashSet;

use chrono::{DateTime, Utc};
use deadpool_diesel::postgres::Pool;
use evian::contracts::starknet::vault::data::indexer::events::{
    DepositEvent, RedeemRequestedEvent, VaultAddress, VaultEvent,
};
use evian::{
    contracts::starknet::vault::StarknetVaultIndexer, utils::indexer::handler::OutputEvent,
};
use pragma_db::models::{
    indexer_state::{IndexerState, IndexerStatus},
    user::User,
    user_position::{NewUserPosition, UserPosition, UserPositionUpdate},
    user_transaction::{NewUserTransaction, TransactionStatus, TransactionType, UserTransaction},
};
use rust_decimal::Decimal;
use starknet::core::types::Felt;
use task_supervisor::{SupervisedTask, TaskError};

use crate::vaults::helpers::felt_to_hex_str;

#[derive(Clone)]
pub struct ExtendedVault {
    pub current_block: u64,
    pub current_timestamp: Option<DateTime<Utc>>,
    pub apibara_api_key: String,
    pub vault_address: Felt,
    pub db_pool: Pool,
}

#[async_trait::async_trait]
impl SupervisedTask for ExtendedVault {
    async fn run(&mut self) -> Result<(), TaskError> {
        // Load the last processed block from the database
        self.load_last_processed_block().await?;

        let vault_indexer = StarknetVaultIndexer::new(
            self.apibara_api_key.clone(),
            HashSet::from([VaultAddress(self.vault_address)]),
            self.current_block,
        );

        // Initialize indexer state with starting block
        self.initialize_indexer_state().await?;

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
                            let (block_number, block_timestamp) =
                            header.map_or_else(|| todo!(), |h| (h.block_number, h.timestamp));

                            let block_timestamp = DateTime::from_timestamp_secs(block_timestamp).unwrap_or_else(|| {
                                panic!("[ExtendedVault] ❌ Invalid timestamp for block {block_number}")
                            });

                            if let Err(e) = self.handle_event(block_number, block_timestamp, event).await {
                                self.record_indexer_error(e.to_string()).await?;
                                return Err(TaskError::from(e));
                            }

                            // Update indexer state
                            self.current_block = block_number;
                            self.current_timestamp = Some(block_timestamp);
                            self.update_indexer_state(block_number, block_timestamp).await?;
                        }
                        OutputEvent::Synced => {
                            tracing::info!("[🔢 Accounting] 🥳 Extended Vault indexer reached the tip of the chain!");
                            // TODO: Should we flag as synced?
                        }
                        // NOTE: Never happens for now. See later when apibara upgrades?
                        OutputEvent::Finalized(_) | OutputEvent::Invalidated(_) => { }
                    }
                }
                res = &mut vault_handle => {
                    let error_msg = format!("😱 Vault indexer stopped: {res:?}");
                    self.record_indexer_error(error_msg.clone()).await?;
                    anyhow::bail!("{error_msg}");
                }
            }
        }
    }
}

impl ExtendedVault {
    // TODO: this should be dynamic based on the vault address
    pub const fn vault_id() -> &'static str {
        "1"
    }

    async fn handle_event(
        &self,
        block_number: u64,
        block_timestamp: DateTime<Utc>,
        event: VaultEvent,
    ) -> Result<(), anyhow::Error> {
        match event {
            VaultEvent::Deposit(deposit) => {
                self.handle_deposit_event(deposit, block_number, block_timestamp)
                    .await?;
            }
            VaultEvent::RedeemRequested(redeem) => {
                self.handle_redeem_requested_event(redeem, block_number, block_timestamp)
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
        block_number: u64,
        block_timestamp: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        tracing::info!("💰 Handling deposit event: {deposit:?}");

        let user_address = felt_to_hex_str(deposit.owner);
        self.ensure_user_exists(user_address.clone()).await?;

        // NOTE: This is the share price at the time of the deposit
        let share_price = if deposit.shares > Decimal::ZERO {
            Some(deposit.assets / deposit.shares)
        } else {
            None
        };

        let new_transaction = NewUserTransaction {
            tx_hash: String::new(), // TODO: Get from the actual transaction hash
            block_number: block_number
                .try_into()
                .expect("[ExtendedVault] 🌯 Block number too large for i64"),
            block_timestamp,
            user_address: user_address.clone(),
            vault_id: Self::vault_id().to_string(),
            type_: TransactionType::Deposit.as_str().to_string(),
            status: TransactionStatus::Confirmed.as_str().to_string(),
            amount: deposit.assets,
            shares_amount: Some(deposit.shares),
            share_price,
            gas_fee: None,
            metadata: None,
        };

        let conn = self.db_pool.get().await?;
        conn.interact(move |conn| UserTransaction::create(&new_transaction, conn))
            .await
            .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Transaction creation failed: {e}"))?
            .map_err(anyhow::Error::from)?;

        let conn = self.db_pool.get().await?;
        conn.interact(move |conn| {
            match UserPosition::find_by_user_and_vault(&user_address, Self::vault_id(), conn) {
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
                        vault_id: Self::vault_id().to_string(),
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
        block_number: u64,
        block_timestamp: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        tracing::info!("💸 Handling redeem requested event: {redeem:?}");

        let user_address = felt_to_hex_str(redeem.owner);
        self.ensure_user_exists(user_address.clone()).await?;

        // NOTE: This is the share price at the time of the redeem request
        let share_price = if redeem.shares > Decimal::ZERO {
            Some(redeem.assets / redeem.shares)
        } else {
            None
        };

        // Create transaction record for withdrawal
        let new_transaction = NewUserTransaction {
            tx_hash: String::new(), // TODO: Get from the actual transaction hash
            block_number: block_number
                .try_into()
                .expect("[ExtendedVault] 🌯 Block number too large for i64"),
            block_timestamp,
            user_address: user_address.clone(),
            vault_id: Self::vault_id().to_string(),
            type_: TransactionType::Withdraw.as_str().to_string(),
            status: TransactionStatus::Pending.as_str().to_string(), // TODO: Change to Confirmed when the redeem is completed?
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
        let conn = self.db_pool.get().await?;
        conn.interact(move |conn| UserTransaction::create(&new_transaction, conn))
            .await
            .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Transaction creation failed: {e}"))?
            .map_err(anyhow::Error::from)?;

        // Second database operation: Update user position
        let conn = self.db_pool.get().await?;
        conn.interact(move |conn| {
            match UserPosition::find_by_user_and_vault(&user_address, Self::vault_id(), conn) {
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
                        Self::vault_id()
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

    /// Ensure user exists in database
    async fn ensure_user_exists(&self, user_address: String) -> Result<(), anyhow::Error> {
        let conn = self.db_pool.get().await?;

        conn.interact(move |conn| {
            User::find_or_create(&user_address, pragma_common::web3::Chain::Starknet, conn)
        })
        .await
        .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Database interaction failed: {e}"))?
        .map_err(anyhow::Error::from)?;

        Ok(())
    }

    /// Load the last processed block from the database
    pub async fn load_last_processed_block(&mut self) -> Result<(), anyhow::Error> {
        let conn = self.db_pool.get().await?;

        let result = conn
            .interact(move |conn| IndexerState::find_by_vault_id(Self::vault_id(), conn))
            .await
            .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Database interaction failed: {e}"))?;

        match result {
            Ok(state) => {
                // Resume from the last processed block + 1
                self.current_block = (state.last_processed_block + 1) as u64;
                tracing::info!(
                    "[ExtendedVault] 📍 Resuming from block {} (last processed: {})",
                    self.current_block,
                    state.last_processed_block
                );
            }
            Err(diesel::result::Error::NotFound) => {
                // No previous state found, start from the configured block
                tracing::info!(
                    "[ExtendedVault] 🆕 No previous state found, starting from block {}",
                    self.current_block
                );
            }
            Err(e) => return Err(anyhow::Error::from(e)),
        }

        Ok(())
    }

    /// Initialize indexer state with the starting block
    async fn initialize_indexer_state(&self) -> Result<(), anyhow::Error> {
        let current_block = self.current_block;
        let conn = self.db_pool.get().await?;

        conn.interact(move |conn| {
            IndexerState::upsert_for_vault(
                Self::vault_id(),
                current_block
                    .try_into()
                    .expect("[ExtendedVault] 🌯 Block number too large for i64"),
                None, // No timestamp for initialization
                Some(IndexerStatus::Active),
                conn,
            )
        })
        .await
        .map_err(|e| {
            anyhow::anyhow!("[ExtendedVault] 🗃️ Indexer state initialization failed: {e}")
        })?
        .map_err(anyhow::Error::from)?;

        Ok(())
    }

    /// Update indexer state with the last processed block
    async fn update_indexer_state(
        &self,
        block_number: u64,
        block_timestamp: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        let conn = self.db_pool.get().await?;

        conn.interact(move |conn| {
            IndexerState::upsert_for_vault(
                Self::vault_id(),
                block_number
                    .try_into()
                    .expect("[ExtendedVault] 🌯 Block number too large for i64"),
                Some(block_timestamp),
                Some(IndexerStatus::Active),
                conn,
            )
        })
        .await
        .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Indexer state update failed: {e}"))?
        .map_err(anyhow::Error::from)?;

        Ok(())
    }

    /// Record an error in the indexer state
    async fn record_indexer_error(&self, error_message: String) -> Result<(), anyhow::Error> {
        let conn = self.db_pool.get().await?;

        let current_block = self.current_block;
        let current_timestamp = self.current_timestamp;
        conn.interact(move |conn| {
            match IndexerState::find_by_vault_id(Self::vault_id(), conn) {
                Ok(state) => state.record_error(error_message, conn).map(|_| ()),
                Err(diesel::result::Error::NotFound) => {
                    // Create new state with error
                    let new_state = pragma_db::models::indexer_state::NewIndexerState {
                        vault_id: Self::vault_id().to_string(),
                        last_processed_block: current_block
                            .try_into()
                            .expect("[ExtendedVault] 🌯 Block number too large for i64"),
                        last_processed_timestamp: current_timestamp,
                        last_error: Some(error_message),
                        last_error_at: Some(Utc::now()),
                        status: Some(IndexerStatus::Error.as_str().to_string()),
                    };
                    IndexerState::create(&new_state, conn).map(|_| ())
                }
                Err(e) => Err(e),
            }
        })
        .await
        .map_err(|e| anyhow::anyhow!("[ExtendedVault] 🗃️ Error recording failed: {e}"))?
        .map_err(anyhow::Error::from)?;

        Ok(())
    }
}
