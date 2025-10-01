use std::collections::HashMap;
use std::sync::Arc;
use std::time::{Duration, Instant};

use anyhow::{Context, Result};
use chrono::Utc;
use rust_decimal::Decimal;
use starknet::core::types::{Felt, U256};
use tokio::time::sleep;
use tokio_util::sync::CancellationToken;
use tracing::{debug, error, info};

use crate::config::RelayerConfig;
use crate::queue::{ExecutionQueue, QueueItem, QueueItemKind};
use crate::repository::{PendingRedeem, RelayerRepository, VaultRecord};
use crate::starknet::StarknetClient;

pub struct RelayerService {
    repository: RelayerRepository,
    starknet: Arc<dyn StarknetClient>,
    config: RelayerConfig,
}

impl RelayerService {
    pub fn new(
        repository: RelayerRepository,
        starknet: Arc<dyn StarknetClient>,
        config: RelayerConfig,
    ) -> Self {
        Self {
            repository,
            starknet,
            config,
        }
    }

    pub async fn run_forever(&self, shutdown: CancellationToken) -> Result<()> {
        let mut vaults: HashMap<String, VaultRuntime> = HashMap::new();
        let mut queue = ExecutionQueue::new();
        let mut last_refresh = Instant::now() - self.config.vault_refresh_interval;

        loop {
            if shutdown.is_cancelled() {
                info!("Relayer shutdown requested");
                break;
            }

            let now = Instant::now();
            if now.duration_since(last_refresh) >= self.config.vault_refresh_interval {
                self.refresh_vaults(&mut vaults).await?;
                last_refresh = now;
            }

            let vault_ids: Vec<String> = vaults.keys().cloned().collect();
            for vault_id in vault_ids {
                if shutdown.is_cancelled() {
                    break;
                }

                let Some(runtime) = vaults.get_mut(&vault_id) else {
                    continue;
                };
                let now = Instant::now();

                if now >= runtime.next_report_check {
                    match self.check_vault_report(runtime, &mut queue).await {
                        Ok(delay) => runtime.next_report_check = Instant::now() + delay,
                        Err(err) => {
                            error!(
                                vault = %runtime.record.id,
                                ?err,
                                "Failed to check vault report status",
                            );
                            runtime.next_report_check =
                                Instant::now() + self.config.aum_worker_error_backoff;
                        }
                    }
                }

                if shutdown.is_cancelled() {
                    break;
                }

                if now >= runtime.next_redeem_check {
                    match self.check_vault_pending_redeems(runtime, &mut queue).await {
                        Ok(delay) => runtime.next_redeem_check = Instant::now() + delay,
                        Err(err) => {
                            error!(
                                vault = %runtime.record.id,
                                ?err,
                                "Failed to check vault pending redeems",
                            );
                            runtime.next_redeem_check =
                                Instant::now() + self.config.redeem_worker_error_backoff;
                        }
                    }
                }
            }

            if shutdown.is_cancelled() {
                break;
            }

            if let Err(err) = self.process_queue(&mut queue).await {
                error!(?err, "Relayer queue processing failed");
            }

            let shutdown_fut = shutdown.clone();
            tokio::select! {
                _ = shutdown_fut.cancelled() => break,
                _ = sleep(self.config.queue_poll_interval) => {}
            }
        }

        Ok(())
    }

    async fn refresh_vaults(&self, vaults: &mut HashMap<String, VaultRuntime>) -> Result<()> {
        let records = self.repository.fetch_live_vaults().await?;

        for record in records {
            vaults
                .entry(record.id.clone())
                .and_modify(|runtime| runtime.record = record.clone())
                .or_insert_with(|| {
                    info!(vault = %record.id, "Discovered vault for relayer monitoring");
                    VaultRuntime {
                        record,
                        next_report_check: Instant::now(),
                        next_redeem_check: Instant::now(),
                    }
                });
        }

        Ok(())
    }

    async fn check_vault_report(
        &self,
        runtime: &VaultRuntime,
        queue: &mut ExecutionQueue,
    ) -> Result<Duration> {
        let contract_address = runtime.record.contract_address;
        let (last_report_timestamp, current_block_timestamp, report_delay) = tokio::join!(
            self.starknet.get_last_report_timestamp(contract_address),
            self.starknet.get_current_block_timestamp(),
            self.starknet.get_report_delay(contract_address),
        );

        let last_report_timestamp =
            last_report_timestamp.context("Fetching last report timestamp")?;
        let current_block_timestamp =
            current_block_timestamp.context("Fetching block timestamp")?;
        let report_delay = report_delay.context("Fetching report delay")?;

        let next_report_time = last_report_timestamp + report_delay;
        let current_time = current_block_timestamp;

        if current_time >= next_report_time {
            let priority = (current_time - next_report_time) as i64;
            info!(
                vault = %runtime.record.id,
                priority,
                "Scheduling AUM report",
            );
            queue.push(QueueItem::new(
                priority,
                QueueItemKind::AumReport {
                    vault_id: runtime.record.id.clone(),
                    contract_address,
                    aum_provider: runtime.record.aum_provider.clone(),
                },
            ));
            Ok(Duration::from_secs(5 * 60))
        } else {
            let remaining_secs = next_report_time - current_time;
            let remaining_secs = remaining_secs.max(1);
            debug!(
                vault = %runtime.record.id,
                remaining_secs,
                "Report not ready yet",
            );
            Ok(Duration::from_secs(remaining_secs))
        }
    }

    async fn check_vault_pending_redeems(
        &self,
        runtime: &VaultRuntime,
        queue: &mut ExecutionQueue,
    ) -> Result<Duration> {
        let Some(indexer_state) = self
            .repository
            .fetch_indexer_state(&runtime.record.id)
            .await?
        else {
            debug!(vault = %runtime.record.id, "Indexer not ready yet");
            return Ok(self.config.redeem_check_interval);
        };

        let current_block = self
            .starknet
            .get_current_block_number()
            .await
            .context("Fetching block number failed")? as i64;

        if current_block - indexer_state.last_processed_block > 10 {
            debug!(
                vault = %runtime.record.id,
                current_block,
                last_indexed = indexer_state.last_processed_block,
                "Indexer is behind head"
            );
            return Ok(Duration::from_secs(60));
        }

        let handled_epoch_len = self
            .starknet
            .get_handled_epoch_len(runtime.record.contract_address)
            .await
            .context("Fetching handled epochs failed")?;

        let pending_redeems = self
            .repository
            .fetch_pending_redeems_below_epoch(&runtime.record.id, handled_epoch_len)
            .await?;

        if pending_redeems.is_empty() {
            debug!(vault = %runtime.record.id, "No pending redeems found");
            return Ok(self.config.redeem_check_interval);
        }

        info!(
            vault = %runtime.record.id,
            pending = pending_redeems.len(),
            "Found pending redeems"
        );

        self.enqueue_redeem_claims(runtime, queue, pending_redeems);

        Ok(self.config.redeem_check_interval)
    }

    fn enqueue_redeem_claims(
        &self,
        runtime: &VaultRuntime,
        queue: &mut ExecutionQueue,
        redeems: Vec<PendingRedeem>,
    ) {
        let mut current_batch: Vec<U256> = Vec::with_capacity(self.config.redeem_batch_size);
        for redeem in redeems {
            current_batch.push(redeem.redeem_id);
            if current_batch.len() == self.config.redeem_batch_size {
                self.push_redeem_batch(runtime, queue, std::mem::take(&mut current_batch));
            }
        }

        if !current_batch.is_empty() {
            self.push_redeem_batch(runtime, queue, current_batch);
        }
    }

    fn push_redeem_batch(
        &self,
        runtime: &VaultRuntime,
        queue: &mut ExecutionQueue,
        batch: Vec<U256>,
    ) {
        let priority = Utc::now().timestamp();
        queue.push(QueueItem::new(
            priority,
            QueueItemKind::RedeemClaim {
                vault_id: runtime.record.id.clone(),
                contract_address: runtime.record.contract_address,
                redeem_ids: batch,
            },
        ));
    }

    async fn process_queue(&self, queue: &mut ExecutionQueue) -> Result<()> {
        while let Some(item) = queue.pop() {
            match &item.kind {
                QueueItemKind::AumReport {
                    vault_id,
                    contract_address,
                    ..
                } => {
                    debug!(vault = %vault_id, "Processing AUM report queue item");
                    if let Err(err) = self
                        .process_aum_report_item(*contract_address, vault_id)
                        .await
                    {
                        error!(vault = %vault_id, ?err, "Failed to execute AUM report");
                    }
                }
                QueueItemKind::RedeemClaim {
                    vault_id,
                    contract_address,
                    redeem_ids,
                } => {
                    debug!(vault = %vault_id, count = redeem_ids.len(), "Processing redeem batch");
                    if let Err(err) = self
                        .process_redeem_claim_item(*contract_address, redeem_ids)
                        .await
                    {
                        error!(vault = %vault_id, ?err, "Failed to execute redeem claim");
                    }

                    if !self.config.redeem_claim_sleep.is_zero() {
                        sleep(self.config.redeem_claim_sleep).await;
                    }
                }
            }
        }

        Ok(())
    }

    async fn process_aum_report_item(&self, contract_address: Felt, vault_id: &str) -> Result<()> {
        let (buffer, aum) = tokio::join!(
            self.starknet.get_buffer(contract_address),
            self.starknet.get_aum(contract_address),
        );

        let buffer = buffer.context("Fetching buffer failed")?;
        let aum = aum.context("Fetching AUM failed")?;
        let liquidity = buffer + aum;

        if liquidity == Decimal::ZERO {
            info!(vault = %vault_id, "Liquidity is zero, skipping report");
            return Ok(());
        }

        let tx_hash = self
            .starknet
            .trigger_report(contract_address, aum)
            .await
            .context("Triggering report failed")?;

        info!(vault = %vault_id, tx_hash, "AUM report executed");
        Ok(())
    }

    async fn process_redeem_claim_item(
        &self,
        contract_address: Felt,
        redeem_ids: &[U256],
    ) -> Result<()> {
        let tx_hash = self
            .starknet
            .claim_redeems(contract_address, redeem_ids.to_vec())
            .await
            .context("Claiming redeems failed")?;

        info!(
            contract = %format!("{contract_address:#x}"),
            tx_hash,
            count = redeem_ids.len(),
            "Redeem batch claimed"
        );

        Ok(())
    }
}

#[derive(Debug)]
struct VaultRuntime {
    record: VaultRecord,
    next_report_check: Instant,
    next_redeem_check: Instant,
}
