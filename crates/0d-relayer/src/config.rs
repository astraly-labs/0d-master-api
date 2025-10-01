use std::time::Duration;

use starknet::core::types::Felt;
use starknet_crypto::Felt as SigningFelt;

/// Configuration for relayer runtime behaviour.
#[derive(Debug, Clone)]
pub struct RelayerConfig {
    pub queue_poll_interval: Duration,
    pub vault_refresh_interval: Duration,
    pub redeem_check_interval: Duration,
    pub aum_worker_error_backoff: Duration,
    pub redeem_worker_error_backoff: Duration,
    pub redeem_claim_sleep: Duration,
    pub redeem_batch_size: usize,
}

impl Default for RelayerConfig {
    fn default() -> Self {
        Self {
            queue_poll_interval: Duration::from_secs(5),
            vault_refresh_interval: Duration::from_secs(60),
            redeem_check_interval: Duration::from_secs(5 * 60),
            aum_worker_error_backoff: Duration::from_secs(5),
            redeem_worker_error_backoff: Duration::from_secs(30),
            redeem_claim_sleep: Duration::from_millis(1_000),
            redeem_batch_size: 10,
        }
    }
}

/// Starknet account configuration used for executing transactions.
#[derive(Debug, Clone)]
pub struct StarknetAccountConfig {
    pub account_address: Felt,
    pub private_key: SigningFelt,
}

impl StarknetAccountConfig {
    pub const fn new(account_address: Felt, private_key: SigningFelt) -> Self {
        Self {
            account_address,
            private_key,
        }
    }
}
