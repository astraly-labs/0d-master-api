use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use utoipa::ToSchema;

/// Vault portfolio snapshot as returned by an individual vault API
/// Example corresponds to Carry Trade vault.
#[derive(Debug, Clone, Serialize, Deserialize, ToSchema)]
pub struct VaultPortfolioDTO {
    pub age_days: String,
    pub last_30d_apr: String,
    pub num_depositors: String,
    pub profit_factor: String,
    pub tvl_in_usd: String,
    pub total_exposure_in_usd: String,
    pub total_pnl: String,
    pub free_collateral: String,
    pub balances: HashMap<String, HashMap<String, String>>,
}
