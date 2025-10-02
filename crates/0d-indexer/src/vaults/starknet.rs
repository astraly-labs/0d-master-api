use std::collections::HashSet;

use chrono::{DateTime, Utc};
use evian::contracts::starknet::vault::StarknetVaultContract;
use evian::contracts::starknet::vault::data::indexer::events::{
    DepositEvent, RedeemClaimedEvent, RedeemRequestedEvent, VaultAddress, VaultEvent,
};
use evian::{
    contracts::starknet::vault::StarknetVaultIndexer, utils::indexer::handler::OutputEvent,
};
use pragma_common::starknet::FallbackProvider;
use rust_decimal::{Decimal, MathematicalOps, dec};
use starknet::core::types::{BlockId, Felt};
use task_supervisor::{SupervisedTask, TaskError};
use zerod_db::ZerodPool;
use zerod_db::models::{
    user::User,
    user_position::{NewUserPosition, UserPosition, UserPositionUpdate},
    user_transaction::{NewUserTransaction, TransactionStatus, TransactionType, UserTransaction},
};

use crate::vaults::helpers::felt_to_hex_str;
use crate::vaults::state::VaultState;

#[derive(Clone)]
pub struct StarknetIndexer {
    pub apibara_api_key: String,
    pub vault_address: Felt,
    pub vault_id: String,
    pub starknet_provider: FallbackProvider,
    pub state: VaultState,
}

#[async_trait::async_trait]
impl SupervisedTask for StarknetIndexer {
    async fn run(&mut self) -> Result<(), TaskError> {
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
            "[StarknetIndexer] 🔌 Connected to the on-chain Vault({})! (from block {})",
            self.vault_id,
            self.state.current_block
        );

        loop {
            tokio::select! {
                Some(output_event) = event_receiver.recv() => {
                    match output_event {
                        OutputEvent::Event { event, event_metadata } => {
                            let (block_number, block_timestamp) =
                            (event_metadata.block_number, event_metadata.timestamp);

                            let block_timestamp = DateTime::from_timestamp_secs(block_timestamp).unwrap_or_else(|| {
                                panic!("[StarknetIndexer] ❌ Invalid timestamp for block {block_number}")
                            });

                            let tx_hash = felt_to_hex_str(event_metadata.transaction_hash);

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
                            self.state.set_indexer_state_synced(&self.vault_id).await?;
                            tracing::info!("[StarknetIndexer] 🥳 Vault({}) reached the tip of the chain!", self.vault_id);
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

impl StarknetIndexer {
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
            VaultEvent::RedeemClaimed(claim) => {
                self.handle_redeem_claimed_event(claim, tx_hash, block_number, block_timestamp)
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
        tracing::info!("[StartknetIndexer] 💰 Handling deposit event with hash: {tx_hash}");

        let user_address = felt_to_hex_str(deposit.owner);
        self.ensure_user_exists(user_address.clone()).await?;

        let vault_contract =
            StarknetVaultContract::new(self.starknet_provider.clone(), self.vault_address);
        let underlying_asset_decimals = Decimal::from(
            vault_contract
                .underlying_asset_decimals(Some(BlockId::Number(block_number)))
                .await?,
        );

        let deposit_shares = deposit.shares / dec!(10).powd(underlying_asset_decimals);
        let deposit_assets = deposit.assets / dec!(10).powd(underlying_asset_decimals);

        // NOTE: This is the share price at the time of the deposit
        let share_price = if deposit_shares > Decimal::ZERO {
            Some(deposit_assets / deposit_shares)
        } else {
            None
        };

        // Check if transaction already exists to avoid duplicates
        let tx_hash_check = tx_hash.clone();
        let tx_exists = self
            .state
            .db_pool
            .interact_with_context(
                format!("check if deposit transaction exists: {tx_hash_check}"),
                move |conn| UserTransaction::exists_by_hash(&tx_hash_check, conn),
            )
            .await?;

        if tx_exists {
            tracing::info!(
                "[StartknetIndexer] ⏭️  Skipping duplicate deposit transaction: {} (block: {})",
                tx_hash,
                block_number
            );
            return Ok(());
        }

        let new_transaction = NewUserTransaction {
            tx_hash,
            block_number: block_number
                .try_into()
                .expect("[StartknetIndexer] 🌯 Block number too large for i64"),
            block_timestamp,
            user_address: user_address.clone(),
            vault_id: self.vault_id.clone(),
            type_: TransactionType::Deposit.as_str().to_string(),
            status: TransactionStatus::Confirmed.as_str().to_string(),
            amount: deposit_assets,
            shares_amount: Some(deposit_shares),
            share_price,
            gas_fee: None,
            metadata: None,
        };

        self.state
            .db_pool
            .interact_with_context(
                format!("create deposit transaction for user: {user_address}"),
                move |conn| UserTransaction::create(&new_transaction, conn),
            )
            .await?;

        let vault_id = self.vault_id.clone();
        self.state
            .db_pool
            .interact_with_context(
                format!("update user position for deposit: user={user_address}, vault={vault_id}"),
                move |conn| {
                    match UserPosition::find_by_user_and_vault(&user_address, &vault_id, conn) {
                        Ok(position) => {
                            // Add deposit to existing position
                            let new_share_balance = position.share_balance + deposit_shares;
                            let new_cost_basis = position.cost_basis + deposit_assets;

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
                                share_balance: deposit_shares,
                                cost_basis: deposit_assets,
                                first_deposit_at: Some(block_timestamp),
                                last_activity_at: Some(block_timestamp),
                            };

                            UserPosition::create(&new_position, conn)
                        }
                        Err(e) => Err(e),
                    }
                },
            )
            .await?;

        Ok(())
    }
    async fn handle_redeem_requested_event(
        &self,
        redeem: RedeemRequestedEvent,
        tx_hash: String,
        block_number: u64,
        block_timestamp: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        tracing::info!("[StarknetIndexer] 💸 Handling redeem requested event with hash: {tx_hash}");

        let user_address = felt_to_hex_str(redeem.owner);
        self.ensure_user_exists(user_address.clone()).await?;

        let vault_contract =
            StarknetVaultContract::new(self.starknet_provider.clone(), self.vault_address);
        let underlying_asset_decimals = Decimal::from(
            vault_contract
                .underlying_asset_decimals(Some(BlockId::Number(block_number)))
                .await?,
        );

        let redeem_shares = redeem.shares / dec!(10).powd(underlying_asset_decimals);
        let redeem_assets = redeem.assets / dec!(10).powd(underlying_asset_decimals);

        // NOTE: This is the share price at the time of the redeem request
        let share_price = if redeem_shares > Decimal::ZERO {
            Some(redeem_assets / redeem_shares)
        } else {
            None
        };

        // Check if transaction already exists to avoid duplicates
        let tx_hash_check = tx_hash.clone();
        let tx_exists = self
            .state
            .db_pool
            .interact_with_context(
                format!("check if withdraw transaction exists: {tx_hash_check}"),
                move |conn| UserTransaction::exists_by_hash(&tx_hash_check, conn),
            )
            .await?;

        if tx_exists {
            tracing::info!(
                "[StarknetIndexer] ⏭️  Skipping duplicate withdraw transaction: {} (block: {})",
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
                .expect("[StartknetIndexer] 🌯 Block number too large for i64"),
            block_timestamp,
            user_address: user_address.clone(),
            vault_id: self.vault_id.clone(),
            type_: TransactionType::Withdraw.as_str().to_string(),
            status: TransactionStatus::Pending.as_str().to_string(),
            amount: redeem_assets,
            shares_amount: Some(redeem_shares),
            share_price,
            gas_fee: None,
            metadata: Some(serde_json::json!({
                "redeem_id": redeem.id.to_string(),
                "epoch": redeem.epoch.to_string(),
                "receiver": redeem.receiver.to_string()
            })),
        };

        // First database operation: Create transaction record
        self.state
            .db_pool
            .interact_with_context(
                format!("create withdraw transaction for user: {user_address}"),
                move |conn| UserTransaction::create(&new_transaction, conn),
            )
            .await?;

        // Second database operation: Update user position
        let vault_id = self.vault_id.clone();
        self.state
            .db_pool
            .interact_with_context(
                format!("update user position for redeem: user={user_address}, vault={vault_id}"),
                move |conn| {
                match UserPosition::find_by_user_and_vault(&user_address, &vault_id, conn) {
                    Ok(position) => {
                        // Reduce share balance for pending redemption
                        let new_share_balance = position.share_balance - redeem_shares;

                        // Ensure we don't go negative
                        let new_share_balance = if new_share_balance < Decimal::ZERO {
                            Decimal::ZERO
                        } else {
                            new_share_balance
                        };

                        let updates = UserPositionUpdate {
                            share_balance: Some(new_share_balance),
                            cost_basis: None,
                            last_activity_at: Some(block_timestamp),
                            updated_at: Some(Utc::now()),
                        };

                        position.update(&updates, conn).map(|_| ())
                    }
                    Err(diesel::result::Error::NotFound) => {
                        tracing::warn!(
                            "[StarknetIndexer] ⏭️ Redeem requested for non-existent position: user={}, vault={}",
                            user_address,
                            vault_id
                        );
                        Ok(())
                    }
                    Err(e) => Err(e),
                }
            },
        )
        .await?;

        Ok(())
    }

    async fn handle_redeem_claimed_event(
        &self,
        redeem_claimed: RedeemClaimedEvent,
        tx_hash: String,
        _block_number: u64,
        block_timestamp: DateTime<Utc>,
    ) -> Result<(), anyhow::Error> {
        tracing::info!("[StartknetIndexer] ✅ Handling redeem claimed event with hash: {tx_hash}");

        let user_address = felt_to_hex_str(redeem_claimed.receiver);
        self.ensure_user_exists(user_address.clone()).await?;

        // First, find the original pending redeem transaction by redeem_id
        let redeem_id = redeem_claimed.id.to_string();
        let vault_id = self.vault_id.clone();

        let transaction = self
            .state
            .db_pool
            .interact_with_context(
                format!("find pending redeem transaction for user: {user_address}"),
                {
                    let user_addr = user_address.clone();
                    let vault_id_check = vault_id.clone();
                    let redeem_id_check = redeem_id.clone();
                    move |conn| {
                        UserTransaction::find_pending_redeem_by_id(
                            &user_addr,
                            &vault_id_check,
                            &redeem_id_check,
                            conn,
                        )
                    }
                },
            )
            .await?;

        // Update the original pending transaction to confirmed status
        self.state
            .db_pool
            .interact_with_context(
                format!(
                    "update transaction status to confirmed: tx_id={}",
                    transaction.id
                ),
                {
                    let tx_id = transaction.id;
                    let new_status = TransactionStatus::Confirmed.as_str().to_string();
                    let actual_amount = redeem_claimed.assets;
                    let new_tx_hash = tx_hash.clone();
                    move |conn| {
                        UserTransaction::update_status_and_amount(
                            tx_id,
                            &new_status,
                            actual_amount,
                            &new_tx_hash,
                            conn,
                        )
                    }
                },
            )
            .await?;

        // Update user position cost_basis to reflect the confirmed redemption
        let user_addr_for_position = user_address.clone();
        let vault_id_for_position = vault_id.clone();
        let redeem_nominal = redeem_claimed.redeem_request_nominal;
        let block_ts = block_timestamp;
        self.state
            .db_pool
            .interact_with_context(
                format!(
                    "update user position cost basis for claim: user={user_address}, vault={vault_id}"),
                move |conn| {
                    match UserPosition::find_by_user_and_vault(
                        &user_addr_for_position,
                        &vault_id_for_position,
                        conn,
                    ) {
                        Ok(position) => {
                            // Calculate the proportion of shares redeemed vs total shares
                            let total_shares_before = position.share_balance + redeem_nominal;
                            let redemption_ratio = if total_shares_before > Decimal::ZERO {
                                redeem_nominal / total_shares_before
                            } else {
                                Decimal::ZERO
                            };

                            // Reduce cost_basis proportionally
                            let cost_basis_reduction = position.cost_basis * redemption_ratio;
                            let new_cost_basis = position.cost_basis - cost_basis_reduction;

                            let updates = UserPositionUpdate {
                                share_balance: None, // Already updated in redeem_requested
                                cost_basis: Some(new_cost_basis),
                                last_activity_at: Some(block_ts),
                                updated_at: Some(Utc::now()),
                            };

                            position.update(&updates, conn)
                        }
                        Err(e) => Err(e),
                    }
                },
            )
            .await?;

        Ok(())
    }

    /// Ensure user exists in database
    async fn ensure_user_exists(&self, user_address: String) -> Result<(), anyhow::Error> {
        self.state
            .db_pool
            .interact_with_context(format!("ensure user exists: {user_address}"), move |conn| {
                User::find_or_create(&user_address, pragma_common::web3::Chain::Starknet, conn)
            })
            .await?;

        Ok(())
    }
}
