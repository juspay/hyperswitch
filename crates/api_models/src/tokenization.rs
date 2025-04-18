use serde::{Deserialize, Serialize};
use common_utils::id_type::GlobalTokenId;

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize)]
pub struct TokenizationResponse {
    pub id: GlobalTokenId,
    pub created_at: i64,
    pub flag: TokenizationFlag,
}

#[cfg(all(feature = "v2", feature = "tokenization_v2"))]
#[derive(Debug, Serialize, Deserialize, strum::Display, strum::EnumString)]
#[strum(serialize_all = "snake_case")]
pub enum TokenizationFlag {
    Enabled,
    Disabled,
} 