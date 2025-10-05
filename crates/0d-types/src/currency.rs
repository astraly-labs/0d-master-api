use serde::{Deserialize, Serialize};
use strum::{AsRefStr, Display, EnumString};
use utoipa::ToSchema;

#[derive(
    Debug,
    Clone,
    Copy,
    Default,
    Serialize,
    Deserialize,
    ToSchema,
    Hash,
    Eq,
    PartialEq,
    Display,
    AsRefStr,
    EnumString,
)]
#[strum(ascii_case_insensitive)]
#[serde(rename_all = "UPPERCASE")]
pub enum Currency {
    #[default]
    USD,
    USDC,
}
